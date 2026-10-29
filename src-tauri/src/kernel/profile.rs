//! The person using the app (D-039). The name is app-wide (registry setting `owner_name`);
//! each workspace adds the job title the owner holds there (`workspace.owner_title`). No
//! account yet: an email arrives with sign-in.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::kernel::ai::translate::SummaryLang;
use crate::kernel::hub::WorkspaceHub;

const NAME: &str = "owner_name";
/// "1" (the default) or "0": new summaries count everyone the meeting involves as attending.
const EVERYONE_ATTENDS: &str = "attendees_everyone";
/// The language the Summary tab offers beside English (D-046); Vietnamese until set.
const SUMMARY_LANGUAGE: &str = "summary_language";

#[derive(Debug, Clone, Serialize, Type)]
pub struct Profile {
    /// "" until the owner sets it.
    pub name: String,
    /// New summaries put everyone the meeting involves in the attendees (the minutes); off,
    /// only the owner and the presenter are, and the owner ticks the rest.
    pub everyone_attends: bool,
    /// The summary's second language, chosen once for every meeting.
    pub summary_language: SummaryLang,
}

#[derive(Debug, Deserialize, Type)]
pub struct ProfilePatch {
    pub name: Option<String>,
    pub everyone_attends: Option<bool>,
    pub summary_language: Option<SummaryLang>,
}

/// The owner as they appear in one workspace's meetings.
#[derive(Debug, Clone, Default)]
pub struct Owner {
    pub name: String,
    /// Their job title in this workspace ("" when not set).
    pub title: String,
    /// The workspace's name.
    pub organisation: String,
}

/// Text the owner typed: no control characters or angle brackets, trimmed, capped.
pub fn clean(text: &str, max: usize) -> String {
    let text: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .filter(|c| !matches!(c, '<' | '>'))
        .collect();
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
}

pub async fn get(hub: &WorkspaceHub) -> Result<Profile> {
    Ok(Profile {
        name: hub.app_setting(NAME).await?.unwrap_or_default(),
        everyone_attends: hub.app_setting(EVERYONE_ATTENDS).await?.as_deref() != Some("0"),
        summary_language: hub
            .app_setting(SUMMARY_LANGUAGE)
            .await?
            .as_deref()
            .and_then(SummaryLang::from_key)
            .unwrap_or(SummaryLang::Vi),
    })
}

pub async fn set(hub: &WorkspaceHub, patch: ProfilePatch) -> Result<Profile> {
    if let Some(name) = patch.name {
        hub.set_app_setting(NAME, &clean(&name, 120)).await?;
    }
    if let Some(on) = patch.everyone_attends {
        hub.set_app_setting(EVERYONE_ATTENDS, if on { "1" } else { "0" })
            .await?;
    }
    if let Some(lang) = patch.summary_language {
        hub.set_app_setting(SUMMARY_LANGUAGE, lang.key()).await?;
    }
    get(hub).await
}

/// The owner in workspace `slug`; None until the name is set.
pub async fn owner(hub: &WorkspaceHub, slug: &str) -> Result<Option<Owner>> {
    let name = get(hub).await?.name;
    if name.is_empty() {
        return Ok(None);
    }
    let workspace = hub.workspace(slug).await?;
    Ok(Some(Owner {
        name,
        title: workspace.owner_title,
        organisation: workspace.name,
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn cleans_what_the_owner_types() {
        assert_eq!(
            super::clean("  Nguyễn  Văn\tTiến <x> ", 120),
            "Nguyễn Văn Tiến x"
        );
        assert_eq!(super::clean("abcdef", 3), "abc");
    }
}
