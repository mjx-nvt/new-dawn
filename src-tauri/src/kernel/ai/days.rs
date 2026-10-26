//! Weekdays and ordinals in live translations (0.7.5). Vietnamese counts the days from Sunday —
//! "thứ hai" (the second) is Monday, "thứ sáu" Friday — and the fast model maps the number
//! instead: "trước thứ sáu" came out "before Saturday", "Friday" came out "thứ Năm", "Thứ hai là…"
//! came out "First is…". It tells a day from an ordinal well, so its choice is kept and only the
//! number is set from the line that was said: when both sides name as many, each mention from the
//! one it pairs with, and only when they're one apart — that slip, not a different day.

use std::ops::Range;
use std::sync::LazyLock;

use regex::{Match, Regex};
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
/// Lines naming more days and ordinals than this are left as they are (every pairing is tried).
const MAX_MENTIONS: usize = 6;

/// "thứ" and a number (words or digits), or "chủ nhật" (group 2, its number group 3); with the
/// word before when it makes "thứ" a thing (group 1: "mọi thứ năm nay", everything this year) and
/// the number word after (group 4: "thứ mười hai" is the 12th, "thứ hai mươi" the 20th).
static VI: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:\b(mọi|những|các|mấy|nhiều)\s+)?\b(thứ\s+(nhất|nhì|hai|ba|tư|bốn|năm|sáu|bảy|bẩy|tám|chín|mười|\d{1,2})|chủ\s+nhật)\b(?:\s+(mươi|mốt|lăm|một|hai|ba|tư|bốn|năm|sáu|bảy|bẩy|tám|chín)\b)?")
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
/// is in the other language). Unchanged unless both sides name as many. A translation may put
/// them in another order ("thứ sáu thứ ba" is "the third Friday"), so the mentions pair up by
/// number, the closest pairing overall — in the order they were said on a tie.
pub fn fix(said: &str, lang: Lang, translation: &str) -> String {
    let said = said.nfc().collect::<String>();
    let translation = translation.nfc().collect::<String>();
    let (vi_text, en_text) = match lang {
        Lang::Vi => (&said, &translation),
        Lang::En => (&translation, &said),
    };
    let vi = vi_mentions(vi_text);
    let en: Vec<Match> = EN.find_iter(en_text).collect();
    if vi.len() != en.len() || vi.len() > MAX_MENTIONS {
        return translation;
    }
    let numbers: Vec<Option<usize>> = vi.iter().map(|m| vi_number(m.as_str())).collect();
    let named: Vec<Option<Named>> = en.iter().map(|m| en_named(m.as_str())).collect();
    let mut edits: Vec<(Range<usize>, String)> = pairing(&numbers, &named)
        .into_iter()
        .enumerate()
        .filter_map(|(i, j)| {
            let (n, named, v, e) = (numbers[i]?, named[j]?, vi[i], en[j]);
            if apart(n, counted(named)) > 1 {
                return None;
            }
            match lang {
                Lang::Vi => {
                    let right = match named {
                        Named::Day(_) => DAYS[as_day(n)?].to_owned(),
                        Named::Nth(_) => cased(ORDINALS[as_nth(n)? - 1], e.as_str()),
                    };
                    (right != e.as_str()).then(|| (e.range(), right))
                }
                Lang::En => (n != counted(named))
                    .then(|| (v.range(), vi_mention(v.as_str(), counted(named)))),
            }
        })
        .collect();
    edits.sort_by_key(|(range, _)| range.start);
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

/// The days and ordinals Vietnamese `text` names — not "mọi thứ năm nay" (a thing) or "thứ mười
/// hai" (past the tenth). "Mọi chủ nhật" is every Sunday.
fn vi_mentions(text: &str) -> Vec<Match<'_>> {
    VI.captures_iter(text)
        .filter(|c| {
            let word = |i: usize| c.get(i).map(|m| m.as_str().to_lowercase());
            let thing = c.get(1).is_some() && c.get(3).is_some();
            let past_tenth = match (word(3), word(4)) {
                (Some(n), Some(next)) => n == "mười" || ["mươi", "mốt", "lăm"].contains(&&*next),
                _ => false,
            };
            !thing && !past_tenth
        })
        .filter_map(|c| c.get(2))
        .collect()
}

/// For each Vietnamese mention, the English one it pairs with: the pairing whose numbers are
/// closest overall, the spoken order on a tie.
fn pairing(numbers: &[Option<usize>], named: &[Option<Named>]) -> Vec<usize> {
    let cost = |order: &Vec<usize>| -> usize {
        order
            .iter()
            .zip(numbers)
            .map(|(&j, n)| match (n, named[j]) {
                (Some(n), Some(named)) => apart(*n, counted(named)),
                _ => 0,
            })
            .sum()
    };
    orders(numbers.len())
        .into_iter()
        .min_by_key(cost)
        .unwrap_or_default()
}

/// Every order of `0..n`, `0, 1, …` first.
fn orders(n: usize) -> Vec<Vec<usize>> {
    if n == 0 {
        return vec![Vec::new()];
    }
    orders(n - 1)
        .into_iter()
        .flat_map(|order| {
            (0..n).rev().map(move |at| {
                let mut order = order.clone();
                order.insert(at, n - 1);
                order
            })
        })
        .collect()
}

/// An English mention as Vietnamese counts it: Monday … Saturday "thứ hai" (2) … "thứ bảy" (7),
/// Sunday "chủ nhật" (0), the nth "thứ n".
fn counted(named: Named) -> usize {
    match named {
        Named::Day(6) => 0,
        Named::Day(d) => d + 2,
        Named::Nth(k) => k,
    }
}

/// How far apart two Vietnamese numbers are, Sunday (0) beside Saturday (7).
fn apart(a: usize, b: usize) -> usize {
    let at = |n: usize| if n == 0 { 8 } else { n };
    at(a).abs_diff(at(b))
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

    #[test]
    fn mentions_pair_up_by_number_not_by_place() {
        let into_en = |said, translation| fix(said, Lang::Vi, translation);
        // English puts the day last; Vietnamese puts the ordinal after the day.
        let right = "Let's hold the third meeting next Monday.";
        assert_eq!(
            into_en("Thứ hai tuần sau mình họp lần thứ ba nhé.", right),
            right
        );
        assert_eq!(
            into_en(
                "Vào thứ sáu sẽ có buổi họp thứ hai với khách hàng.",
                "The second meeting with the client will be on Saturday."
            ),
            "The second meeting with the client will be on Friday."
        );
        let said = "Hội đồng họp vào thứ sáu thứ ba hàng tháng.";
        let right = "The board meets on the third Friday of every month.";
        assert_eq!(into_en(said, right), right);
        assert_eq!(
            into_en(
                said,
                "The board meets on the third Saturday of every month."
            ),
            right
        );
        let right = "The draft is due Thursday, and the deadline is Friday.";
        assert_eq!(
            into_en(
                "Thứ sáu là hạn chót, nhưng thứ năm phải có bản nháp.",
                right
            ),
            right
        );
        // The same the other way: English said, the Vietnamese translation puts the day first.
        let english = "The board meets on the third Friday of every month.";
        let right = "Hội đồng họp vào thứ Sáu thứ ba hằng tháng.";
        assert_eq!(fix(english, Lang::En, right), right);
        assert_eq!(
            fix(
                english,
                Lang::En,
                "Hội đồng họp vào thứ Năm thứ ba hằng tháng."
            ),
            right
        );
        // A number word after the day that isn't part of it: Saturday at eight.
        assert_eq!(
            into_en("Họp thứ bảy tám giờ nhé.", "Meet Sunday at eight."),
            "Meet Saturday at eight."
        );
    }

    #[test]
    fn things_and_big_ordinals_are_no_days() {
        let into_en = |said, translation| fix(said, Lang::Vi, translation);
        // "mọi thứ năm nay": everything this year; "first" came from "lần đầu tiên".
        for (said, translation) in [
            (
                "Mọi thứ năm nay đều tốt, lần đầu tiên doanh thu vượt kế hoạch.",
                "Everything's been good this year; for the first time, revenue exceeded the plan.",
            ),
            (
                "Đầu tiên, mọi thứ năm nay đều ổn.",
                "First, everything's been fine this year.",
            ),
            (
                "Mọi thứ năm nay chốt vào Friday nhé.",
                "Everything this year is finalized on Friday.",
            ),
            (
                "Đây là lần thứ mười hai, và là lần đầu tiên ở Hà Nội.",
                "This is the twelfth time, and the first in Hanoi.",
            ),
        ] {
            assert_eq!(into_en(said, translation), translation, "{said}");
        }
        let translation = "Chờ một chút, mọi thứ năm nay đều ổn.";
        assert_eq!(
            fix(
                "Just a second, everything this year looks fine.",
                Lang::En,
                translation
            ),
            translation
        );
        // Every Sunday is still a day.
        assert_eq!(
            into_en("Mọi chủ nhật đều nghỉ.", "Every Saturday is off."),
            "Every Sunday is off."
        );
        // Not the slip it fixes (one apart): a different day is left to the model.
        assert_eq!(
            into_en("Thứ sáu nhé.", "See you Monday."),
            "See you Monday."
        );
    }
}
