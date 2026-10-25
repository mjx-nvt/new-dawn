use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::error::AppError;

/// Any workspace was created, changed, archived, deleted or chosen. Every window refreshes.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct WorkspacesChanged {}

/// A native menu item that the UI has to act on.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "snake_case")]
pub enum MenuAction {
    SwitchWorkspace,
    ToggleSidebar,
    OpenTenantSetup,
    ImportAudio,
    /// File → New Recording (⌘R) or the tray item: start when idle, stop when recording.
    ToggleRecording,
    /// New Dawn → Run Setup…: the first-run welcome steps again.
    RunSetup,
    /// View → Icon-only Sidebar (⌃⌘S): labels on/off.
    IconSidebar,
    /// Workspace → Workspace Settings… (⌥⌘,): brand, meeting types, people of the current one.
    OpenWorkspace,
    /// View → Back / Forward (⌘[ / ⌘]): in-app history.
    GoBack,
    GoForward,
}

/// Ask an already-open Settings window to show a tab.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct OpenSettingsTab {
    pub tab: crate::kernel::windows::SettingsTab,
}

/// Bytes so far for a model download (every ~250 ms).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ModelDownloadProgress {
    pub id: String,
    pub downloaded: u32,
    pub total: u32,
    pub bytes_per_sec: u32,
}

/// A model download ended: `error` is None on success.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ModelDownloadFinished {
    pub id: String,
    pub error: Option<AppError>,
}

/// A transcription job moved forward (`percent` 0–100).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct TranscriptionProgress {
    pub workspace_slug: String,
    pub meeting_id: String,
    pub stage: crate::kernel::ai::whisper::Stage,
    pub percent: u32,
}

/// A transcription job ended: `error` is None on success; code `cancelled` when stopped.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct TranscriptionFinished {
    pub workspace_slug: String,
    pub meeting_id: String,
    pub error: Option<AppError>,
}

/// Recorder phase / system-audio state changed.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct RecorderStatusChanged(pub crate::recorder::session::RecorderStatus);

/// 10 Hz meter while recording: RMS 0–1 for the mic ("you") and the call ("them").
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct RecorderLevel {
    pub you: f32,
    pub them: f32,
    pub elapsed_ms: u32,
}

/// A summary job moved forward (`tokens` generated so far in the current call).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct SummaryProgress {
    pub workspace_slug: String,
    pub meeting_id: String,
    pub stage: crate::modules::meetings::summarise::SummaryStage,
    pub part: u32,
    pub parts: u32,
    pub tokens: u32,
}

/// A summary job ended: `error` is None on success; code `cancelled` when stopped.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct SummaryFinished {
    pub workspace_slug: String,
    pub meeting_id: String,
    pub error: Option<AppError>,
}

pub fn notify_workspaces_changed(app: &AppHandle) {
    let event = WorkspacesChanged {};
    if let Err(err) = event.emit(app) {
        tracing::warn!("could not emit WorkspacesChanged: {err}");
    }
}

/// Ollama is downloading Qwen (onboarding): the current layer's progress, in MB.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct OllamaPullProgress {
    pub model: String,
    pub status: String,
    pub completed_mb: u32,
    pub total_mb: u32,
}

/// The Qwen download ended: `error` is None on success.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct OllamaPullFinished {
    pub model: String,
    pub error: Option<AppError>,
}

/// A workspace's brand pack changed (colours, logo, reset): every window re-applies it.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct BrandChanged {
    pub slug: String,
}
