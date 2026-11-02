//! Schedules (D-068): Twenty's "every N minutes / hours / days" and a five-field cron
//! (minute hour day month weekday; `*`, `*/n`, lists and ranges), in this Mac's own time.

use chrono::{DateTime, Datelike, Duration, Local, Timelike};

/// Whether a cron field admits a value.
fn admits(field: &str, value: u32, min: u32, max: u32) -> bool {
    field.split(',').any(|part| {
        let part = part.trim();
        let (range, step) = match part.split_once('/') {
            Some((r, s)) => (r, s.parse::<u32>().unwrap_or(1).max(1)),
            None => (part, 1),
        };
        let (lo, hi) = if range == "*" {
            (min, max)
        } else if let Some((a, b)) = range.split_once('-') {
            match (a.parse::<u32>(), b.parse::<u32>()) {
                (Ok(a), Ok(b)) => (a, b),
                _ => return false,
            }
        } else {
            match range.parse::<u32>() {
                Ok(n) => (n, if step > 1 { max } else { n }),
                Err(_) => return false,
            }
        };
        value >= lo && value <= hi && (value - lo).is_multiple_of(step)
    })
}

/// The first minute after `from` the expression admits, within a year; none for a bad expression.
pub fn next_after(expr: &str, from: DateTime<Local>) -> Option<DateTime<Local>> {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return None;
    }
    let mut t = from.with_second(0)?.with_nanosecond(0)? + Duration::minutes(1);
    for _ in 0..(366 * 24 * 60) {
        let (minute, hour, day, month, weekday) = (t.minute(), t.hour(), t.day(), t.month(), t.weekday().num_days_from_sunday());
        if !admits(fields[3], month, 1, 12) {
            // Skip to the first day of the next month.
            let (y, m) = if month == 12 { (t.year() + 1, 1) } else { (t.year(), month + 1) };
            t = Local.with_ymd_and_hms(y, m, 1, 0, 0, 0).single()?;
            continue;
        }
        if !admits(fields[2], day, 1, 31) || !admits(fields[4], weekday, 0, 6) {
            t = (t + Duration::days(1)).with_hour(0)?.with_minute(0)?;
            continue;
        }
        if !admits(fields[1], hour, 0, 23) {
            t = (t + Duration::hours(1)).with_minute(0)?;
            continue;
        }
        if admits(fields[0], minute, 0, 59) {
            return Some(t);
        }
        t += Duration::minutes(1);
    }
    None
}

use chrono::TimeZone;

/// The next time of an interval: every N minutes, hours or days from `from`; none past the
/// calendar's end.
pub fn next_interval(every: u32, unit: &str, from: DateTime<Local>) -> Option<DateTime<Local>> {
    let n = i64::from(every.max(1));
    let span = match unit {
        "HOURS" => Duration::try_hours(n),
        "DAYS" => Duration::try_days(n),
        _ => Duration::try_minutes(n),
    }?;
    from.checked_add_signed(span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_and_intervals_find_the_next_minute() {
        let from = Local.with_ymd_and_hms(2026, 11, 2, 8, 30, 0).unwrap();
        assert_eq!(next_after("0 9 * * *", from).unwrap(), Local.with_ymd_and_hms(2026, 11, 2, 9, 0, 0).unwrap());
        assert_eq!(next_after("*/15 * * * *", from).unwrap(), Local.with_ymd_and_hms(2026, 11, 2, 8, 45, 0).unwrap());
        assert_eq!(next_after("0 9 * * 1", from).unwrap(), Local.with_ymd_and_hms(2026, 11, 2, 9, 0, 0).unwrap(), "2 Nov 2026 is a Monday and 09:00 is still ahead");
        let tuesday = Local.with_ymd_and_hms(2026, 11, 3, 8, 30, 0).unwrap();
        assert_eq!(next_after("0 9 * * 1", tuesday).unwrap(), Local.with_ymd_and_hms(2026, 11, 9, 9, 0, 0).unwrap(), "the next Monday");
        assert!(next_after("bad", from).is_none());
        assert_eq!(next_interval(2, "HOURS", from), Local.with_ymd_and_hms(2026, 11, 2, 10, 30, 0).single());
        assert_eq!(next_interval(u32::MAX, "DAYS", from), None, "past the calendar's end");
    }
}
