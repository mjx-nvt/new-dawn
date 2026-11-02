//! Plane's filters for work items (D-077): a rule a field — the title, the state or its group, the
//! priority, assignees, labels, who made it, who follows it, whom its comments mention, the start,
//! due, created and updated dates — with Plane's operators (any of, none of, contains; is,
//! before, after, on or before, on or after, between, not, empty, not empty); the rules all hold,
//! a rule's values any one. Dates are days (YYYY-MM-DD) or words for the owner's days — today,
//! tomorrow, yesterday, this, next or last week (from the owner's first day of the week), this,
//! next or last month. Turned into SQL with every value bound.

use anyhow::{Result, anyhow};
use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;
use crate::kernel::formats::WeekStart;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PmRule {
    pub field: String,
    pub op: String,
    pub values: Vec<String>,
}

pub const FIELDS: [&str; 13] = ["title", "state", "state_group", "priority", "assignees", "labels", "created_by", "subscribers", "mentions", "start", "due", "created", "updated"];
const LIST_OPS: [&str; 2] = ["in", "not_in"];
const DATE_OPS: [&str; 9] = ["is", "before", "after", "on_or_before", "on_or_after", "between", "not", "empty", "not_empty"];

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

/// The first day of `day`'s week.
fn week_of(day: NaiveDate, start: WeekStart) -> NaiveDate {
    let back = match start {
        WeekStart::Monday => day.weekday().num_days_from_monday(),
        WeekStart::Sunday => day.weekday().num_days_from_sunday(),
        WeekStart::Saturday => (day.weekday().num_days_from_sunday() + 1) % 7,
    };
    day - Duration::days(i64::from(back))
}

fn month_of(day: NaiveDate, months: i32) -> (NaiveDate, NaiveDate) {
    let total = day.year() * 12 + day.month0() as i32 + months;
    let first = NaiveDate::from_ymd_opt(total.div_euclid(12), total.rem_euclid(12) as u32 + 1, 1).unwrap_or(day);
    let next = NaiveDate::from_ymd_opt((total + 1).div_euclid(12), (total + 1).rem_euclid(12) as u32 + 1, 1).unwrap_or(day);
    (first, next - Duration::days(1))
}

/// A value as the days it covers: a day itself, or a word's span.
fn span(value: &str, today: NaiveDate, start: WeekStart) -> Result<(NaiveDate, NaiveDate)> {
    let week = |n: i64| {
        let first = week_of(today, start) + Duration::days(7 * n);
        (first, first + Duration::days(6))
    };
    Ok(match value {
        "today" => (today, today),
        "tomorrow" => (today + Duration::days(1), today + Duration::days(1)),
        "yesterday" => (today - Duration::days(1), today - Duration::days(1)),
        "this_week" => week(0),
        "next_week" => week(1),
        "last_week" => week(-1),
        "this_month" => month_of(today, 0),
        "next_month" => month_of(today, 1),
        "last_month" => month_of(today, -1),
        day => {
            let d = NaiveDate::parse_from_str(day, "%Y-%m-%d").map_err(|_| invalid(format!("\"{day}\" isn't a day")))?;
            (d, d)
        }
    })
}

/// The rules as SQL to add to a work item query (` AND …`, over `i` the item and `s` its state),
/// with their values in order.
pub fn sql(rules: &[PmRule], today: NaiveDate, start: WeekStart) -> Result<(String, Vec<String>)> {
    if rules.len() > 50 {
        return Err(invalid("At most 50 filters"));
    }
    if rules.iter().any(|r| r.values.len() > 200) {
        return Err(invalid("A filter takes at most 200 values"));
    }
    let mut out = String::new();
    let mut binds: Vec<String> = Vec::new();
    for r in rules {
        if !FIELDS.contains(&r.field.as_str()) {
            return Err(invalid(format!("Work items can't be filtered by \"{}\"", r.field)));
        }
        let marks = |n: usize| vec!["?"; n].join(", ");
        let values: Vec<String> = r.values.iter().map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()).collect();
        match r.field.as_str() {
            "title" => {
                if r.op != "contains" {
                    return Err(invalid("A title filter is \"contains\""));
                }
                let Some(words) = values.first() else { continue };
                out.push_str(" AND i.name LIKE ? ESCAPE '\\'");
                binds.push(format!("%{}%", words.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")));
            }
            "state" | "state_group" | "priority" | "created_by" => {
                if !LIST_OPS.contains(&r.op.as_str()) {
                    return Err(invalid(format!("\"{}\" isn't a way to filter {}", r.op, r.field)));
                }
                if values.is_empty() {
                    continue;
                }
                let column = match r.field.as_str() {
                    "state" => "i.state_id",
                    "state_group" => "s.state_group",
                    "priority" => "i.priority",
                    _ => "i.created_by",
                };
                out.push_str(&format!(" AND {column} {}IN ({})", if r.op == "not_in" { "NOT " } else { "" }, marks(values.len())));
                binds.extend(values);
            }
            "assignees" | "labels" | "subscribers" | "mentions" => {
                if !LIST_OPS.contains(&r.op.as_str()) {
                    return Err(invalid(format!("\"{}\" isn't a way to filter {}", r.op, r.field)));
                }
                if values.is_empty() {
                    continue;
                }
                let (table, column) = match r.field.as_str() {
                    "assignees" => ("pm_item_assignee x WHERE x.item_id = i.id", "x.who"),
                    "labels" => ("pm_item_label x WHERE x.item_id = i.id", "x.label_id"),
                    "subscribers" => ("pm_item_subscriber x WHERE x.item_id = i.id", "x.who"),
                    _ => ("pm_comment c JOIN pm_mention x ON x.comment_id = c.id WHERE c.item_id = i.id", "x.who"),
                };
                // "none": no assignee / no label at all.
                let none = values.iter().any(|v| v == "none");
                let named: Vec<String> = values.into_iter().filter(|v| v != "none").collect();
                let mut any = Vec::new();
                if !named.is_empty() {
                    any.push(format!("EXISTS (SELECT 1 FROM {table} AND {column} IN ({}))", marks(named.len())));
                }
                if none {
                    any.push(format!("NOT EXISTS (SELECT 1 FROM {table})"));
                }
                let clause = any.join(" OR ");
                out.push_str(&if r.op == "not_in" { format!(" AND NOT ({clause})") } else { format!(" AND ({clause})") });
                binds.extend(named);
            }
            _ => {
                if !DATE_OPS.contains(&r.op.as_str()) {
                    return Err(invalid(format!("\"{}\" isn't a way to filter a date", r.op)));
                }
                let column = match r.field.as_str() {
                    "start" => "i.start_date",
                    "due" => "i.target_date",
                    // Stored in UTC: compared as this Mac's day, as the tables show them.
                    "created" => "date(i.created_at, 'localtime')",
                    _ => "date(i.updated_at, 'localtime')",
                };
                match r.op.as_str() {
                    "empty" => out.push_str(&format!(" AND {column} IS NULL")),
                    "not_empty" => out.push_str(&format!(" AND {column} IS NOT NULL")),
                    op => {
                        let Some(first) = values.first() else { continue };
                        let (from, to) = span(first, today, start)?;
                        let (from, to) = if op == "between" {
                            let last = values.get(1).ok_or_else(|| invalid("Between needs two days"))?;
                            (from, span(last, today, start)?.1)
                        } else {
                            (from, to)
                        };
                        let (sql, days): (String, Vec<NaiveDate>) = match op {
                            "is" | "between" => (format!(" AND {column} BETWEEN ? AND ?"), vec![from, to]),
                            "not" => (format!(" AND ({column} IS NULL OR {column} NOT BETWEEN ? AND ?)"), vec![from, to]),
                            "before" => (format!(" AND {column} < ?"), vec![from]),
                            "after" => (format!(" AND {column} > ?"), vec![to]),
                            "on_or_before" => (format!(" AND {column} <= ?"), vec![to]),
                            _ => (format!(" AND {column} >= ?"), vec![from]),
                        };
                        out.push_str(&sql);
                        binds.extend(days.into_iter().map(|d| d.to_string()));
                    }
                }
            }
        }
    }
    Ok((out, binds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weeks_and_months_are_the_owners() {
        let day = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();
        // A Wednesday.
        assert_eq!(week_of(day(2026, 11, 4), WeekStart::Monday), day(2026, 11, 2));
        assert_eq!(week_of(day(2026, 11, 4), WeekStart::Sunday), day(2026, 11, 1));
        assert_eq!(week_of(day(2026, 11, 4), WeekStart::Saturday), day(2026, 10, 31));
        assert_eq!(month_of(day(2026, 12, 15), 1), (day(2027, 1, 1), day(2027, 1, 31)));
        assert_eq!(month_of(day(2026, 1, 15), -1), (day(2025, 12, 1), day(2025, 12, 31)));
        assert_eq!(span("this_week", day(2026, 11, 4), WeekStart::Sunday).unwrap(), (day(2026, 11, 1), day(2026, 11, 7)));
        assert!(span("someday", day(2026, 11, 4), WeekStart::Monday).is_err());
    }
}
