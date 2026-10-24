//! Microphone permission via AVCaptureDevice; System Audio Recording has no query API, so the
//! recorder detects it (silent tap) and the UI links to the right System Settings pane.

use anyhow::{Result, anyhow};
use cidre::{av, blocks};
use serde::Serialize;
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    Granted,
    Denied,
    NotDetermined,
    Restricted,
}

pub fn microphone() -> Result<PermissionState> {
    let status = av::CaptureDevice::authorization_status_for_media_type(av::MediaType::audio())
        .map_err(|e| anyhow!("reading microphone permission: {e:?}"))?;
    Ok(match status {
        av::AuthorizationStatus::Authorized => PermissionState::Granted,
        av::AuthorizationStatus::Denied => PermissionState::Denied,
        av::AuthorizationStatus::Restricted => PermissionState::Restricted,
        _ => PermissionState::NotDetermined,
    })
}

/// Shows the macOS prompt if the user hasn't decided yet; returns the resulting state.
/// (cidre's async variant isn't `Send`, so the completion handler feeds a channel on a
/// blocking thread instead.)
pub async fn request_microphone() -> Result<PermissionState> {
    if microphone()? != PermissionState::NotDetermined {
        return microphone();
    }
    tokio::task::spawn_blocking(|| -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel::<bool>();
        let mut handler = blocks::SendBlock::new1(move |granted: bool| {
            let _ = tx.send(granted);
        });
        av::CaptureDevice::request_access_for_media_type_ch(av::MediaType::audio(), &mut handler)
            .map_err(|e| anyhow!("requesting microphone permission: {e:?}"))?;
        // The user answers the macOS prompt; give them time, but never hang forever.
        let _ = rx.recv_timeout(std::time::Duration::from_secs(300));
        Ok(())
    })
    .await??;
    microphone()
}

/// Privacy & Security → Screen & System Audio Recording.
pub const SYSTEM_AUDIO_SETTINGS: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";
/// Privacy & Security → Microphone.
pub const MICROPHONE_SETTINGS: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone";
