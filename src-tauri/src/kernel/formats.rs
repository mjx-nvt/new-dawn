//! The owner's formats (D-072, Twenty's Experience settings): the time zone times are shown in,
//! how a date, a time and a number read, and the day a week starts — app-wide, in the registry.
//! The windows format with them; Today's week starts on the chosen day.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;

const KEY: &str = "formats";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DateFormat {
    /// "24 Oct 2026" — New Dawn's own.
    #[default]
    DayMonthName,
    /// "24/10/2026".
    DayFirst,
    /// "10/24/2026".
    MonthFirst,
    /// "2026-10-24".
    YearFirst,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum TimeFormat {
    /// "14:05".
    #[default]
    Hours24,
    /// "2:05 PM".
    Hours12,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum NumberFormat {
    /// "1,234.56".
    #[default]
    CommasAndDot,
    /// "1.234,56".
    DotsAndComma,
    /// "1 234,56".
    SpacesAndComma,
    /// "1’234.56".
    ApostrophesAndDot,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WeekStart {
    #[default]
    Monday,
    Sunday,
    Saturday,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Formats {
    /// An IANA time zone ("Asia/Ho_Chi_Minh"); empty: this Mac's.
    pub time_zone: String,
    pub date: DateFormat,
    pub time: TimeFormat,
    pub number: NumberFormat,
    pub week_start: WeekStart,
}

pub async fn get(hub: &WorkspaceHub) -> Result<Formats> {
    Ok(hub.app_setting(KEY).await?.and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default())
}

/// Keeps the formats. A zone is checked for its shape here and against the system's zones in
/// the window that picks it.
pub async fn set(hub: &WorkspaceHub, formats: Formats) -> Result<Formats> {
    let zone = formats.time_zone.trim().to_owned();
    if zone.len() > 64 || zone.starts_with('/') || !zone.chars().all(|c| c.is_ascii_alphanumeric() || "/_+-".contains(c)) {
        return Err(AppError::invalid("That isn't a time zone").into());
    }
    let formats = Formats { time_zone: zone, ..formats };
    hub.set_app_setting(KEY, &serde_json::to_string(&formats)?).await?;
    Ok(formats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_are_kept_and_read_back() {
        let root = std::env::temp_dir().join(format!("newdawn-formats-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            assert_eq!(get(&hub).await.unwrap(), Formats::default(), "New Dawn's own until changed");
            let chosen = Formats { time_zone: " Asia/Ho_Chi_Minh ".into(), date: DateFormat::YearFirst, time: TimeFormat::Hours12, number: NumberFormat::DotsAndComma, week_start: WeekStart::Sunday };
            let kept = set(&hub, chosen).await.unwrap();
            assert_eq!(kept.time_zone, "Asia/Ho_Chi_Minh");
            assert_eq!(get(&hub).await.unwrap(), kept);
            assert_eq!(serde_json::to_value(&kept).unwrap()["date"], "yearFirst");
            assert!(set(&hub, Formats { time_zone: "Mars/Olympus Mons; drop".into(), ..Formats::default() }).await.is_err());
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
