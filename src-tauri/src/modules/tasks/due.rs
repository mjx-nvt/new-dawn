//! Due phrases → a local calendar day (0.7.3): "Friday", "tuần sau", "15/11" are counted from the
//! meeting's own day; a phrase that names no day ("ASAP", "before the board meeting") stays as
//! said. Vietnamese keywords keep their accents — folded, "tháng sáu" (June) would read as "tháng
//! sau" (next month) and "ngày mốt" (the day after tomorrow) as "một" (one).

use std::sync::LazyLock;

use chrono::{DateTime, Datelike, Days, Local, Months, NaiveDate, Weekday};
use regex::Regex;
use unicode_normalization::UnicodeNormalization;

/// What a due phrase says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Due {
    /// Nothing was set ("", "Not set", "TBD").
    None,
    On(NaiveDate),
    /// Said, but not a day that can be counted; shown as said.
    Text,
}

/// Phrases that mean no date at all.
const NO_DATE: &[&str] = &[
    "",
    "not set",
    "not specified",
    "none",
    "tbd",
    "tbc",
    "n/a",
    "-",
    "—",
    "chưa xác định",
    "chưa có",
];

/// The meeting's calendar day where the Mac is: a call at 06:00 in Hanoi is that day, not the
/// day before in UTC.
pub fn local_day(held_on: &str) -> Option<NaiveDate> {
    DateTime::parse_from_rfc3339(held_on)
        .ok()
        .map(|t| t.with_timezone(&Local).date_naive())
}

/// The day `said` names, counted from `day` (the meeting's local day). First match wins: no
/// date, an explicit date, a relative day, a weekday, an offset, a period; else the phrase.
pub fn parse_due(said: &str, day: NaiveDate) -> Due {
    let text = said.nfc().collect::<String>().to_lowercase();
    let text = text.trim().trim_end_matches('.').trim();
    if NO_DATE.contains(&text) {
        return Due::None;
    }
    explicit(text, day)
        .or_else(|| relative(text, day))
        .map_or(Due::Text, Due::On)
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("a valid pattern")
}

const MONTHS: &str = "(january|february|march|april|may|june|july|august|september|october|november|december|jan|feb|mar|apr|jun|jul|aug|sept|sep|oct|nov|dec)";

static ISO: LazyLock<Regex> = LazyLock::new(|| re(r"\b(\d{4})-(\d{1,2})-(\d{1,2})\b"));
static NUMERIC: LazyLock<Regex> =
    LazyLock::new(|| re(r"\b(\d{1,2})[/.](\d{1,2})(?:[/.](\d{4}|\d{2}))?\b"));
static VIETNAMESE: LazyLock<Regex> =
    LazyLock::new(|| re(r"\b(\d{1,2})\s+tháng\s+(\d{1,2})\b(?:\s+năm\s+(\d{4}))?"));
static DAY_MONTH: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"\b(\d{{1,2}})(?:st|nd|rd|th)?\s+(?:of\s+)?{MONTHS}\b\.?(?:,?\s+(\d{{4}}))?"
    ))
});
static MONTH_DAY: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"\b{MONTHS}\b\.?\s+(\d{{1,2}})(?:st|nd|rd|th)?\b(?:,?\s+(\d{{4}}))?"
    ))
});
static NTH: LazyLock<Regex> =
    LazyLock::new(|| re(r"\b(\d{1,2})(?:st|nd|rd|th)\b|\bngày\s+(\d{1,2})\b"));
static QUARTER: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?:q|quý\s*)([1-4]|iv|iii|ii|i)\b"));
static OFFSET: LazyLock<Regex> = LazyLock::new(|| {
    let count = "(\\d+|an?|one|two|three|four|five|six|seven|eight|nine|ten|một|hai|ba|bốn|năm|sáu|bảy|tám|chín|mười)";
    re(&format!(
        r"\b(?:in|within|trong(?:\s+vòng)?)\s+{count}\s+(days?|weeks?|months?|ngày|tuần|tháng)\b|\b{count}\s+(ngày|tuần|tháng)\s+(?:nữa|tới)\b"
    ))
});

/// Weekday names and the ways Vietnamese writes them ("thứ 6", "T6"); whole words only.
const WEEKDAYS: &[(&str, Weekday)] = &[
    ("monday", Weekday::Mon),
    ("mon", Weekday::Mon),
    ("thứ hai", Weekday::Mon),
    ("thứ 2", Weekday::Mon),
    ("t2", Weekday::Mon),
    ("tuesday", Weekday::Tue),
    ("tues", Weekday::Tue),
    ("tue", Weekday::Tue),
    ("thứ ba", Weekday::Tue),
    ("thứ 3", Weekday::Tue),
    ("t3", Weekday::Tue),
    ("wednesday", Weekday::Wed),
    ("wed", Weekday::Wed),
    ("thứ tư", Weekday::Wed),
    ("thứ 4", Weekday::Wed),
    ("t4", Weekday::Wed),
    ("thursday", Weekday::Thu),
    ("thurs", Weekday::Thu),
    ("thur", Weekday::Thu),
    ("thu", Weekday::Thu),
    ("thứ năm", Weekday::Thu),
    ("thứ 5", Weekday::Thu),
    ("t5", Weekday::Thu),
    ("friday", Weekday::Fri),
    ("fri", Weekday::Fri),
    ("thứ sáu", Weekday::Fri),
    ("thứ 6", Weekday::Fri),
    ("t6", Weekday::Fri),
    ("saturday", Weekday::Sat),
    ("sat", Weekday::Sat),
    ("thứ bảy", Weekday::Sat),
    ("thứ 7", Weekday::Sat),
    ("t7", Weekday::Sat),
    ("sunday", Weekday::Sun),
    ("sun", Weekday::Sun),
    ("chủ nhật", Weekday::Sun),
    ("cn", Weekday::Sun),
];

/// Short forms that are also words or names ("doanh thu", "Sun Group", "CN Hà Nội"): a day only
/// alone or beside a word that dates it ("by Thu", "trước T6", "T3 tuần sau").
const SHORT_DAYS: &[&str] = &[
    "thu", "sat", "sun", "cn", "t2", "t3", "t4", "t5", "t6", "t7",
];
const BEFORE_DAY: &[&str] = &[
    "by", "before", "on", "eod", "next", "this", "trước", "vào", "hạn", "sáng", "trưa", "chiều",
    "tối",
];
const AFTER_DAY: &[&str] = &["eod", "next", "this", "tuần", "này", "tới"];
/// The parts of a day that make "mai" tomorrow and not Ms Mai ("sáng mai", "chờ chị Mai").
const PART_OF_DAY: &[&str] = &["ngày", "sáng", "trưa", "chiều", "tối"];
/// Nouns an ordinal follows ("tuần thứ 2", "buổi họp thứ hai", "lần thứ 3"): there "thứ" is
/// "-th", not a weekday.
const COUNTED: &[&str] = &[
    "tuần", "buổi", "họp", "lần", "vòng", "đợt", "phiên", "kỳ", "kì",
];

/// A written date: ISO, day/month (day first, as in Vietnam and the UK; "11/15" month first only
/// when day first can't be a date), "15 tháng 11", "15 Nov", "Nov 15", or "the 12th" / "ngày 12".
fn explicit(text: &str, day: NaiveDate) -> Option<NaiveDate> {
    let n = |c: &regex::Captures, i: usize| c.get(i).and_then(|m| m.as_str().parse::<u32>().ok());
    if let Some(c) = ISO.captures(text) {
        return NaiveDate::from_ymd_opt(n(&c, 1)? as i32, n(&c, 2)?, n(&c, 3)?);
    }
    // "9.30 sáng" is a time; month first ("11/15") only with slashes — "9.30" is never 30 Sep.
    for c in NUMERIC.captures_iter(text) {
        if clock(&text[c.get(0)?.end()..]) {
            continue;
        }
        let (a, b, year) = (n(&c, 1)?, n(&c, 2)?, n(&c, 3));
        return dated(day, a, b, year)
            .or_else(|| dated(day, b, a, year).filter(|_| c[0].contains('/')));
    }
    if let Some(c) = VIETNAMESE.captures(text) {
        return dated(day, n(&c, 1)?, n(&c, 2)?, n(&c, 3));
    }
    if let Some(c) = DAY_MONTH.captures(text) {
        return dated(day, n(&c, 1)?, month(&c[2])?, n(&c, 3));
    }
    if let Some(c) = MONTH_DAY.captures(text) {
        return dated(day, n(&c, 2)?, month(&c[1])?, n(&c, 3));
    }
    let c = NTH.captures(text)?;
    // "ngày 15 tháng sáu": a month in words is no "next 15th".
    if text[c.get(0)?.end()..].trim_start().starts_with("tháng") {
        return None;
    }
    let nth = n(&c, 1).or_else(|| n(&c, 2))?;
    let first = day.with_day(1)?;
    (0..12).find_map(|k| {
        first
            .checked_add_months(Months::new(k))?
            .with_day(nth)
            .filter(|d| *d >= day)
    })
}

/// The next word is a time of day: "9.30 sáng", "8.15 pm".
fn clock(rest: &str) -> bool {
    matches!(
        rest.split(|c: char| !c.is_alphanumeric())
            .find(|w| !w.is_empty()),
        Some("am" | "pm" | "h" | "giờ" | "sáng" | "trưa" | "chiều" | "tối")
    )
}

fn month(name: &str) -> Option<u32> {
    [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ]
    .iter()
    .position(|m| name.starts_with(m))
    .map(|i| i as u32 + 1)
}

/// Day `d` of month `m`, in year `year` as written ("26" is 2026); else the meeting's year — or
/// the next one when that would be more than 30 days before the meeting ("05/01" said in December).
fn dated(day: NaiveDate, d: u32, m: u32, year: Option<u32>) -> Option<NaiveDate> {
    if let Some(y) = year {
        return NaiveDate::from_ymd_opt(if y < 100 { 2000 + y } else { y } as i32, m, d);
    }
    let this = NaiveDate::from_ymd_opt(day.year(), m, d)?;
    if this < day.checked_sub_days(Days::new(30))? {
        NaiveDate::from_ymd_opt(day.year() + 1, m, d)
    } else {
        Some(this)
    }
}

/// Days said in words. "Next week" is that week's Friday: `due` is a deadline, and Monday would
/// make "sometime next week" overdue from Tuesday. Weeks start on Monday.
fn relative(text: &str, day: NaiveDate) -> Option<NaiveDate> {
    // Whole words, accents kept: " mai " is never inside another word.
    let words = format!(
        " {} ",
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    );
    let has = |phrase: &str| words.contains(&format!(" {phrase} "));
    let any = |phrases: &[&str]| phrases.iter().any(|p| has(p));
    let beside = |short: &str, before: &[&str], after: &[&str]| {
        words.trim() == short
            || before.iter().any(|w| has(&format!("{w} {short}")))
            || after.iter().any(|w| has(&format!("{short} {w}")))
    };
    let plus = |n: u64| day.checked_add_days(Days::new(n));
    let monday = day.week(Weekday::Mon).first_day();
    let next_monday = monday.checked_add_days(Days::new(7))?;
    let month_end = |later: u32| {
        day.with_day(1)?
            .checked_add_months(Months::new(later + 1))?
            .pred_opt()
    };

    // "mai mốt" is "some day", not the day after tomorrow.
    if any(&["day after tomorrow", "ngày kia"]) || beside("mốt", PART_OF_DAY, &[]) {
        return plus(2);
    }
    if has("tomorrow") || beside("mai", PART_OF_DAY, &[]) {
        return plus(1);
    }
    if any(&[
        "today",
        "tonight",
        "hôm nay",
        "sáng nay",
        "trưa nay",
        "chiều nay",
        "tối nay",
        "trong ngày",
    ]) {
        return Some(day);
    }
    let named = |name: &str| {
        if SHORT_DAYS.contains(&name) {
            beside(name, BEFORE_DAY, AFTER_DAY)
        } else {
            has(name)
                && !(name.starts_with("thứ ")
                    && COUNTED.iter().any(|c| has(&format!("{c} {name}"))))
        }
    };
    if let Some(&(_, weekday)) = WEEKDAYS.iter().find(|(name, _)| named(name)) {
        let wanted = u64::from(weekday.num_days_from_monday());
        // "next Friday", "thứ Sáu tuần sau": that day of next week. Else the next one after the
        // meeting — a Friday said on a Friday is next week's ("today" is said as today).
        if any(&["next", "tuần sau", "tuần tới"]) {
            return next_monday.checked_add_days(Days::new(wanted));
        }
        let ahead = (wanted + 7 - u64::from(day.weekday().num_days_from_monday())) % 7;
        return plus(if ahead == 0 { 7 } else { ahead });
    }
    if let Some(c) = OFFSET.captures(&words) {
        let (count, unit) = match (c.get(1), c.get(2)) {
            (Some(count), Some(unit)) => (count, unit),
            _ => (c.get(3)?, c.get(4)?),
        };
        let count = number(count.as_str())?;
        return match unit.as_str() {
            u if u.starts_with("day") || u == "ngày" => plus(count.into()),
            u if u.starts_with("week") || u == "tuần" => plus(7 * u64::from(count)),
            _ => day.checked_add_months(Months::new(count)),
        };
    }
    if any(&[
        "early next week",
        "beginning of next week",
        "đầu tuần sau",
        "đầu tuần tới",
    ]) {
        return Some(next_monday);
    }
    if any(&["next week", "tuần sau", "tuần tới"]) {
        return next_monday.checked_add_days(Days::new(4));
    }
    if any(&[
        "this week",
        "end of week",
        "end of the week",
        "eow",
        "cuối tuần",
        "trong tuần",
        "tuần này",
    ]) {
        // Said at a weekend meeting: the coming Friday.
        let friday = monday.checked_add_days(Days::new(4))?;
        return if friday < day {
            friday.checked_add_days(Days::new(7))
        } else {
            Some(friday)
        };
    }
    if any(&[
        "early next month",
        "beginning of next month",
        "đầu tháng sau",
        "đầu tháng tới",
    ]) {
        return day.with_day(1)?.checked_add_months(Months::new(1));
    }
    if any(&["next month", "tháng sau", "tháng tới"]) {
        return month_end(1);
    }
    if any(&[
        "end of month",
        "end of the month",
        "eom",
        "this month",
        "cuối tháng",
        "trong tháng",
        "tháng này",
    ]) {
        return month_end(0);
    }
    if let Some(c) = QUARTER.captures(&words) {
        let q = match &c[1] {
            "i" => 1,
            "ii" => 2,
            "iii" => 3,
            "iv" => 4,
            digit => digit.parse().ok()?,
        };
        // The quarter's last day; next year's when this year's is over.
        let end = |year| {
            NaiveDate::from_ymd_opt(year, q * 3, 1)?
                .checked_add_months(Months::new(1))?
                .pred_opt()
        };
        let this = end(day.year())?;
        return if this < day {
            end(day.year() + 1)
        } else {
            Some(this)
        };
    }
    if any(&[
        "end of year",
        "end of the year",
        "eoy",
        "year end",
        "this year",
        "cuối năm",
        "trong năm",
        "năm nay",
    ]) {
        return NaiveDate::from_ymd_opt(day.year(), 12, 31);
    }
    // A time of day alone is today ("EOD"); with a day ("EOD Friday") the day won above.
    if any(&["eod", "cob", "end of day", "end of the day"]) {
        return Some(day);
    }
    None
}

/// "2", "two", "a", "hai" → 2 / 1.
fn number(word: &str) -> Option<u32> {
    const EN: [&str; 10] = [
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    ];
    const VI: [&str; 10] = [
        "một", "hai", "ba", "bốn", "năm", "sáu", "bảy", "tám", "chín", "mười",
    ];
    if matches!(word, "a" | "an") {
        return Some(1);
    }
    word.parse().ok().or_else(|| {
        EN.iter()
            .chain(&VI)
            .position(|w| *w == word)
            .map(|i| i as u32 % 10 + 1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_due_phrases_from_the_meeting_day() {
        let monday = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
        let on = |m, d| Due::On(NaiveDate::from_ymd_opt(2026, m, d).unwrap());
        let table = [
            ("", Due::None),
            ("Not set", Due::None),
            ("TBD", Due::None),
            ("N/A", Due::None),
            ("Chưa xác định", Due::None),
            ("ASAP", Due::Text),
            ("ongoing", Due::Text),
            ("before the board meeting", Due::Text),
            ("sau chuyến công tác", Due::Text),
            ("today", on(10, 26)),
            ("EOD", on(10, 26)),
            ("tonight", on(10, 26)),
            ("hôm nay", on(10, 26)),
            ("chiều nay", on(10, 26)),
            ("tomorrow", on(10, 27)),
            ("ngày mai", on(10, 27)),
            ("sáng mai", on(10, 27)),
            ("day after tomorrow", on(10, 28)),
            ("ngày kia", on(10, 28)),
            ("ngày mốt", on(10, 28)),
            ("Friday", on(10, 30)),
            ("Fri", on(10, 30)),
            ("by Friday", on(10, 30)),
            ("EOD Friday", on(10, 30)),
            ("thứ Sáu", on(10, 30)),
            ("thứ 6", on(10, 30)),
            ("T6", on(10, 30)),
            ("Monday", on(11, 2)),
            ("next Friday", on(11, 6)),
            ("thứ Sáu tuần sau", on(11, 6)),
            ("this week", on(10, 30)),
            ("end of week", on(10, 30)),
            ("EOW", on(10, 30)),
            ("cuối tuần", on(10, 30)),
            ("trong tuần", on(10, 30)),
            ("next week", on(11, 6)),
            ("tuần sau", on(11, 6)),
            ("tuần tới", on(11, 6)),
            ("early next week", on(11, 2)),
            ("đầu tuần sau", on(11, 2)),
            ("end of month", on(10, 31)),
            ("EOM", on(10, 31)),
            ("cuối tháng", on(10, 31)),
            ("next month", on(11, 30)),
            ("tháng sau", on(11, 30)),
            ("tháng sáu", Due::Text), // June, not "next month"
            ("end of year", on(12, 31)),
            ("cuối năm", on(12, 31)),
            ("Q4", on(12, 31)),
            ("quý 4", on(12, 31)),
            ("in 2 weeks", on(11, 9)),
            ("2 tuần nữa", on(11, 9)),
            ("trong 2 tuần", on(11, 9)),
            ("15/11", on(11, 15)),
            ("15.11", on(11, 15)),
            ("ngày 15/11", on(11, 15)),
            ("15 tháng 11", on(11, 15)),
            ("15 Nov", on(11, 15)),
            ("Nov 15", on(11, 15)),
            ("November 15th", on(11, 15)),
            ("2026-11-15", on(11, 15)),
            ("the 12th", on(11, 12)),
            ("ngày 12", on(11, 12)),
            ("ngày 15 tháng sáu", Due::Text),
            ("thứ Sáu, 30/10", on(10, 30)), // the written date wins
            // Words and names that look like days, and times that look like dates.
            ("Sau nghiệm thu", Due::Text),
            ("Sau khi chốt doanh thu", Due::Text),
            ("Chờ chị Mai duyệt", Due::Text),
            ("mai mốt", Due::Text),
            ("Sau buổi làm việc với Sun Group", Due::Text),
            ("CN Hà Nội duyệt", Due::Text),
            ("tuần thứ 2 của tháng 11", Due::Text), // ordinals: the 2nd week, not Monday
            ("sau buổi họp thứ hai", Due::Text),
            ("lần thứ 3", Due::Text),
            ("trước 9.30", Due::Text),
            ("trước 9.30 sáng mai", on(10, 27)),
            ("8.10 sáng thứ 6", on(10, 30)),
            ("mai", on(10, 27)),
            ("Thu", on(10, 29)),
            ("by Thu", on(10, 29)),
            ("trước T6", on(10, 30)),
            ("T3 tuần sau", on(11, 3)),
            ("CN", on(11, 1)),
            ("11/15", on(11, 15)), // month first, with slashes only
        ];
        for (said, want) in table {
            assert_eq!(parse_due(said, monday), want, "{said:?}");
        }
        let december = NaiveDate::from_ymd_opt(2026, 12, 20).unwrap();
        assert_eq!(
            parse_due("05/01", december),
            Due::On(NaiveDate::from_ymd_opt(2027, 1, 5).unwrap()),
            "early next year"
        );
        assert_eq!(
            local_day("2026-10-25T23:30:00.000Z").map(|d| d.year()),
            Some(2026)
        );
    }
}
