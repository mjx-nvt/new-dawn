use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::crm::{self, Commitment, CommitmentStatus, PersonCard, PersonDossier};
use crate::modules::crm_agent::{self, CrmProposal};

/// The CRM agent's proposals waiting for the owner — all, or those about one record (D-072).
#[tauri::command]
#[specta::specta]
pub async fn list_crm_proposals(hub: State<'_, WorkspaceHub>, slug: String, about: Option<String>) -> Result<Vec<CrmProposal>, AppError> {
    Ok(crm_agent::waiting(&hub.pool(&slug)?, about.as_deref()).await?)
}

/// Writes a proposal into the CRM, as the agent (once).
#[tauri::command]
#[specta::specta]
pub async fn apply_crm_proposal(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<CrmProposal, AppError> {
    Ok(crm_agent::apply(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn dismiss_crm_proposal(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<CrmProposal, AppError> {
    Ok(crm_agent::dismiss(&hub.pool(&slug)?, &id).await?)
}

/// A meeting's ⋯ → Find CRM proposals: reads its summary now. How many it proposed.
#[tauri::command]
#[specta::specta]
pub async fn find_crm_proposals(app: AppHandle, slug: String, meeting_id: String) -> Result<u32, AppError> {
    Ok(crm_agent::propose(&app, &slug, &meeting_id, true).await?)
}

/// The People page: everyone in the people memory, with last contact and commitments.
#[tauri::command]
#[specta::specta]
pub async fn list_people_cards(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<PersonCard>, AppError> {
    Ok(crm::people(&hub, &slug).await?)
}

/// Confirmed commitments of someone the people memory doesn't know (an organisation, a name).
#[tauri::command]
#[specta::specta]
pub async fn list_unlinked_commitments(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<Commitment>, AppError> {
    Ok(crm::unlinked(&hub.pool(&slug)?).await?)
}

/// The newest dismissed, for Undo.
#[tauri::command]
#[specta::specta]
pub async fn list_dismissed_commitments(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<Commitment>, AppError> {
    Ok(crm::dismissed(&hub.pool(&slug)?).await?)
}

/// Commitments from new summaries, waiting for the owner.
#[tauri::command]
#[specta::specta]
pub async fn list_waiting_commitments(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<Commitment>, AppError> {
    Ok(crm::waiting(&hub.pool(&slug)?).await?)
}

/// A person's page: meetings, commitments, notes.
#[tauri::command]
#[specta::specta]
pub async fn get_person(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<PersonDossier, AppError> {
    Ok(crm::dossier(&hub.pool(&slug)?, &id).await?)
}

/// Confirm, kept, dropped or dismissed.
#[tauri::command]
#[specta::specta]
pub async fn set_commitment_status(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
    status: CommitmentStatus,
) -> Result<Commitment, AppError> {
    Ok(crm::set_status(&hub.pool(&slug)?, &id, status).await?)
}

/// "Look through earlier meetings": their others' action items become commitments to confirm.
#[tauri::command]
#[specta::specta]
pub async fn collect_commitments(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<u32, AppError> {
    Ok(crm::collect(&hub, &slug).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_person_notes(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
    notes: String,
) -> Result<(), AppError> {
    Ok(crm::set_notes(&hub.pool(&slug)?, &id, &notes).await?)
}
