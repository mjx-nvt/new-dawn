//! The email body is a rendering of the summary — no second AI call (D-026 in plan.md).

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};

use super::summarise::{SectionKind, SummaryDoc};
use crate::kernel::ai::whisper::Language;

pub struct EmailContext<'a> {
    pub workspace: &'a str,
    pub title: &'a str,
    /// ISO-8601 UTC, e.g. "2026-10-24T07:46:46.000Z".
    pub held_on: &'a str,
    pub language: Language,
}

/// Subject `[Tenant] Title — 24 Oct 2026` and a Markdown body: greeting, overview, the sections in
/// order (decisions and action items from the structured lists), sign-off.
pub fn render(doc: &SummaryDoc, ctx: &EmailContext) -> (String, String) {
    let vi = ctx.language == Language::Vi;
    let subject = format!(
        "[{}] {} — {}",
        ctx.workspace,
        ctx.title,
        display_date(ctx.held_on)
    );
    let mut body = String::from(if vi {
        "Kính gửi anh chị,\n\n"
    } else {
        "Dear all,\n\n"
    });
    for section in &doc.sections {
        let text = section.markdown.trim();
        if section.kind == SectionKind::Overview {
            if !text.is_empty() {
                body.push_str(text);
                body.push_str("\n\n");
            }
            continue;
        }
        if text.is_empty() {
            continue;
        }
        body.push_str(&format!("**{}**\n\n{}\n\n", section.title, text));
    }
    body.push_str(if vi {
        "Trân trọng,"
    } else {
        "Best regards,"
    });
    (subject, body)
}

/// "2026-10-24T…" → "24 Oct 2026" (the UTC calendar date).
// ponytail: UTC date; a call before 07:00 in Vietnam shows the previous day. Use the local offset
// when chrono arrives.
fn display_date(iso: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let parts: Vec<&str> = iso.get(..10).unwrap_or("").split('-').collect();
    match parts.as_slice() {
        [y, m, d] => match m.parse::<usize>() {
            Ok(m @ 1..=12) => format!("{} {} {y}", d.trim_start_matches('0'), MONTHS[m - 1]),
            _ => iso.to_owned(),
        },
        _ => iso.to_owned(),
    }
}

/// Markdown → HTML for the clipboard and the Summary tab. Raw HTML in the model's output is
/// escaped (rendered as text), and links/images keep only their text — a prompt-injected
/// tracking pixel or `javascript:` link can't reach Mail.
pub fn to_html(markdown: &str) -> String {
    let parser = Parser::new_ext(
        markdown,
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH,
    )
    .filter_map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => Some(Event::Text(raw)),
        Event::Start(Tag::Link { .. } | Tag::Image { .. })
        | Event::End(TagEnd::Link | TagEnd::Image) => None,
        other => Some(other),
    });
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::meetings::summarise::{ActionItem, Decision, Section};

    fn doc() -> SummaryDoc {
        let s = |kind, title: &str, md: &str| Section {
            kind,
            title: title.into(),
            markdown: md.into(),
        };
        SummaryDoc {
            sections: vec![
                s(
                    SectionKind::Overview,
                    "Overview",
                    "We reviewed the pipeline.",
                ),
                s(
                    SectionKind::KeyPoints,
                    "Key points",
                    "- Twelve active deals",
                ),
                s(SectionKind::Decisions, "Decisions", "- Sign by Friday"),
                s(
                    SectionKind::ActionItems,
                    "Action items",
                    "- **Minh** — circulate the agenda — *Friday*",
                ),
                s(SectionKind::NextSteps, "Next steps", ""),
            ],
            decisions: vec![Decision {
                text: "Sign by Friday".into(),
                ..Decision::default()
            }],
            action_items: vec![ActionItem {
                owner: "Minh".into(),
                item: "circulate the agenda".into(),
                due: "Friday".into(),
            }],
            ..SummaryDoc::default()
        }
    }

    #[test]
    fn renders_the_email() {
        let ctx = EmailContext {
            workspace: "VIFC",
            title: "Partner call",
            held_on: "2026-10-04T08:00:00.000Z",
            language: Language::En,
        };
        let (subject, body) = render(&doc(), &ctx);
        assert_eq!(subject, "[VIFC] Partner call — 4 Oct 2026");
        assert!(body.starts_with("Dear all,\n\nWe reviewed the pipeline.\n\n**Key points**"));
        assert!(body.contains("**Minh** — circulate the agenda — *Friday*"));
        assert!(!body.contains("Next steps"), "empty sections are left out");
        assert!(body.ends_with("Best regards,"));
        let (_, vi) = render(
            &doc(),
            &EmailContext {
                language: Language::Vi,
                ..ctx
            },
        );
        assert!(vi.starts_with("Kính gửi anh chị,") && vi.ends_with("Trân trọng,"));
    }

    #[test]
    fn html_escapes_raw_markup() {
        let out = to_html("**ok** <script>alert(1)</script>\n\n| a | b |\n|---|---|\n| 1 | 2 |");
        assert!(out.contains("<strong>ok</strong>"));
        assert!(!out.contains("<script>"), "{out}");
        assert!(out.contains("<table>"));
        let out = to_html("see [site](javascript:alert(1)) ![px](p.png)");
        assert!(
            !out.contains("href") && !out.contains("<img") && out.contains("site"),
            "{out}"
        );
        // Block-level HTML, autolinks and code-fence info strings stay inert too.
        for nasty in [
            "<img src=x onerror=alert(1)>\n",
            "<script>\nalert(1)\n</script>\n",
            "<javascript:alert(1)>",
            "```\"><b>x\ncode\n```",
        ] {
            let out = to_html(nasty);
            assert!(
                !out.contains("<img")
                    && !out.contains("<script")
                    && !out.contains("href")
                    && !out.contains("<b>"),
                "{nasty:?} → {out}"
            );
        }
    }
}
