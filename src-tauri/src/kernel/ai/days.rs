//! Weekdays and ordinals in live translations (0.7.5). Vietnamese counts the days from Sunday —
//! "thứ hai" (the second) is Monday, "thứ sáu" Friday — and the fast model maps the number
//! instead: "trước thứ sáu" came out "before Saturday", "Friday" came out "thứ Năm", "Thứ hai là…"
//! came out "First is…". It tells a day from an ordinal well, so its choice is kept and only the
//! number is set from the line that was said, mention by mention, when both sides name as many.

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;
use unicode_normalization::UnicodeNormalization;

use crate::modules::meetings::Lang;

const DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const ORDINALS: [&str; 10] = [
    "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth",
];
/// "thứ" + these: the 1st … 10th, and Monday (2) … Saturday (7).
const NUMBERS: [&str; 10] = [
    "nhất", "hai", "ba", "tư", "năm", "sáu", "bảy", "tám", "chín", "mười",
];

/// "thứ" and a number (words or digits), or "chủ nhật".
static VI: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:thứ\s+(?:nhất|nhì|hai|ba|tư|bốn|năm|sáu|bảy|bẩy|tám|chín|mười|\d{1,2})|chủ\s+nhật)\b")
        .expect("a valid pattern")
});
static EN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)\b(?:{}|{})\b",
        DAYS.join("|"),
        ORDINALS.join("|")
    ))
    .expect("a valid pattern")
});

/// What an English word names: a weekday (0 = Monday) or an ordinal (1 = first).
#[derive(Clone, Copy)]
enum Named {
    Day(usize),
    Nth(usize),
}

/// `translation` with each weekday and ordinal set from `said` (heard in `lang`; the translation
/// is in the other language), in order. Unchanged unless both sides name as many.
pub fn fix(said: &str, lang: Lang, translation: &str) -> String {
    let said = said.nfc().collect::<String>();
    let translation = translation.nfc().collect::<String>();
    let edits: Vec<(Range<usize>, String)> = match lang {
        Lang::Vi => {
            let from: Vec<_> = VI.find_iter(&said).map(|m| vi_number(m.as_str())).collect();
            let to: Vec<_> = EN.find_iter(&translation).collect();
            if from.len() != to.len() {
                return translation;
            }
            from.into_iter()
                .zip(to)
                .filter_map(|(n, m)| {
                    let right = match en_named(m.as_str())? {
                        Named::Day(_) => DAYS[as_day(n?)?].to_owned(),
                        Named::Nth(_) => cased(ORDINALS[as_nth(n?)? - 1], m.as_str()),
                    };
                    (right != m.as_str()).then(|| (m.range(), right))
                })
                .collect()
        }
        Lang::En => {
            let from: Vec<_> = EN.find_iter(&said).map(|m| en_named(m.as_str())).collect();
            let to: Vec<_> = VI.find_iter(&translation).collect();
            if from.len() != to.len() {
                return translation;
            }
            from.into_iter()
                .zip(to)
                .filter_map(|(named, m)| {
                    let right = match named? {
                        Named::Day(6) => 0,
                        Named::Day(d) => d + 2,
                        Named::Nth(k) => k,
                    };
                    (vi_number(m.as_str()) != Some(right))
                        .then(|| (m.range(), vi_mention(m.as_str(), right)))
                })
                .collect()
        }
    };
    let mut out = String::with_capacity(translation.len());
    let mut last = 0;
    for (range, with) in edits {
        out.push_str(&translation[last..range.start]);
        out.push_str(&with);
        last = range.end;
    }
    out.push_str(&translation[last..]);
    out
}

/// The number a Vietnamese mention says: 0 for "chủ nhật", n for "thứ n".
fn vi_number(mention: &str) -> Option<usize> {
    let word = mention.split_whitespace().nth(1)?.to_lowercase();
    match word.as_str() {
        "nhật" => Some(0),
        "nhì" => Some(2),
        "bốn" => Some(4),
        "bẩy" => Some(7),
        w => w
            .parse()
            .ok()
            .filter(|n| (1..=10).contains(n))
            .or_else(|| NUMBERS.iter().position(|n| *n == w).map(|i| i + 1)),
    }
}

/// A Vietnamese number as a weekday (0 = Monday … 6 = Sunday): "thứ hai" … "thứ bảy", "chủ nhật".
fn as_day(n: usize) -> Option<usize> {
    match n {
        0 => Some(6),
        2..=7 => Some(n - 2),
        _ => None,
    }
}

fn as_nth(n: usize) -> Option<usize> {
    (1..=10).contains(&n).then_some(n)
}

fn en_named(word: &str) -> Option<Named> {
    let same = |w: &&str| w.eq_ignore_ascii_case(word);
    DAYS.iter()
        .position(same)
        .map(Named::Day)
        .or_else(|| ORDINALS.iter().position(same).map(|i| Named::Nth(i + 1)))
}

/// `mention` rewritten to say `n` (0 = "chủ nhật"), in its own case and digits or words.
fn vi_mention(mention: &str, n: usize) -> String {
    let mut words = mention.split_whitespace();
    let (first, number) = (
        words.next().unwrap_or_default(),
        words.next().unwrap_or_default(),
    );
    if n == 0 {
        return cased("chủ nhật", first);
    }
    let number = if number.chars().all(|c| c.is_ascii_digit()) {
        n.to_string()
    } else {
        cased(NUMBERS[n - 1], number)
    };
    format!("{} {number}", cased("thứ", first))
}

/// `word` with its first letter in the case of `like`'s.
fn cased(word: &str, like: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) if like.chars().next().is_some_and(char::is_uppercase) => {
            first.to_uppercase().chain(chars).collect()
        }
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_and_ordinals_follow_the_vietnamese() {
        let into_en = |said, translation| fix(said, Lang::Vi, translation);
        assert_eq!(
            into_en(
                "Mình chốt gửi báo cáo trước thứ sáu nhé.",
                "Let's submit the report before Saturday."
            ),
            "Let's submit the report before Friday."
        );
        assert_eq!(
            into_en(
                "Thứ năm này anh có rảnh không?",
                "Are you free this Friday?"
            ),
            "Are you free this Thursday?"
        );
        assert_eq!(
            into_en(
                "Từ thứ 2 đến thứ sáu bên em đều làm việc.",
                "We work from Tuesday to Saturday."
            ),
            "We work from Monday to Friday."
        );
        // An ordinal stays an ordinal, with the right number.
        assert_eq!(
            into_en(
                "Thứ hai là ngân sách, thứ ba là nhân sự.",
                "First is the budget, second is personnel."
            ),
            "Second is the budget, third is personnel."
        );
        assert_eq!(into_en("Chủ nhật nghỉ.", "Sunday off."), "Sunday off.");
        // Not as many on both sides: left as it is.
        assert_eq!(
            into_en("Đầu tiên, họp vào thứ sáu.", "First, we meet on Saturday."),
            "First, we meet on Saturday."
        );
        // Decomposed accents (as some sources write them) still match.
        let nfd = "trước thứ sáu".nfd().collect::<String>();
        assert_eq!(into_en(&nfd, "by Saturday"), "by Friday");
    }

    #[test]
    fn days_into_vietnamese_follow_the_english() {
        let into_vi = |said, translation| fix(said, Lang::En, translation);
        assert_eq!(
            into_vi(
                "Let's send the report before Friday.",
                "Gửi báo cáo trước thứ Năm nhé."
            ),
            "Gửi báo cáo trước thứ Sáu nhé."
        );
        assert_eq!(
            into_vi("Are you free this Thursday?", "Thứ Tư bạn có rảnh không?"),
            "Thứ Năm bạn có rảnh không?"
        );
        assert_eq!(
            into_vi(
                "Can we move it to Tuesday or Wednesday?",
                "Dời sang thứ hai hay thứ ba được không?"
            ),
            "Dời sang thứ ba hay thứ tư được không?"
        );
        assert_eq!(into_vi("Friday works.", "Thứ 5 được."), "Thứ 6 được.");
        assert_eq!(
            into_vi("See you Sunday.", "Hẹn gặp thứ Bảy."),
            "Hẹn gặp chủ nhật."
        );
        assert_eq!(
            into_vi("Second, the budget.", "Thứ nhất, ngân sách."),
            "Thứ hai, ngân sách."
        );
        assert_eq!(into_vi("Monday is fine.", "Thứ hai được."), "Thứ hai được.");
    }
}
