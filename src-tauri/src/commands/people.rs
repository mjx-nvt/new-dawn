use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::people::{self, Person, PersonInput};

#[tauri::command]
#[specta::specta]
pub async fn list_people(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<Person>, AppError> {
    Ok(people::list(&hub.pool(&slug)?).await?)
}

/// Adds (no id) or updates a remembered person.
#[tauri::command]
#[specta::specta]
pub async fn save_person(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    person: PersonInput,
) -> Result<Person, AppError> {
    Ok(people::save(&hub.pool(&slug)?, person).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_person(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<(), AppError> {
    Ok(people::delete(&hub.pool(&slug)?, &id).await?)
}

/// `other` becomes another spelling of `keep`.
#[tauri::command]
#[specta::specta]
pub async fn merge_people(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    keep: String,
    other: String,
) -> Result<Person, AppError> {
    Ok(people::merge(&hub.pool(&slug)?, &keep, &other).await?)
}
