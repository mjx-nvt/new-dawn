use std::sync::OnceLock;

use serde::Serialize;
use specta::Type;
use tauri::WebviewWindow;
use window_vibrancy::{
    apply_liquid_glass, apply_vibrancy, LiquidGlassOptions, NSGlassEffectViewStyle,
    NSVisualEffectMaterial,
};

/// Matches `--radius-window` in src/styles/tokens.css.
const WINDOW_RADIUS: f64 = 12.0;

#[derive(Clone, Copy, Debug, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum GlassStatus {
    LiquidGlass,
    Vibrancy,
    None,
}

static STATUS: OnceLock<GlassStatus> = OnceLock::new();

/// Native glass behind the whole window: Liquid Glass on macOS 26+, Sidebar vibrancy below.
/// Must run on the main thread (window-vibrancy returns `NotMainThread` otherwise).
pub fn apply_shell_glass(window: &WebviewWindow) {
    let options = LiquidGlassOptions::new(NSGlassEffectViewStyle::Regular).radius(WINDOW_RADIUS);
    let status = match apply_liquid_glass(window, options) {
        Ok(()) => GlassStatus::LiquidGlass,
        Err(liquid_err) => {
            tracing::warn!("liquid glass unavailable ({liquid_err}); falling back to vibrancy");
            match apply_vibrancy(window, NSVisualEffectMaterial::Sidebar, None, Some(WINDOW_RADIUS)) {
                Ok(()) => GlassStatus::Vibrancy,
                Err(err) => {
                    tracing::warn!("vibrancy unavailable ({err}); window stays transparent");
                    GlassStatus::None
                }
            }
        }
    };
    let _ = STATUS.set(status);
}

/// What `apply_shell_glass` achieved; `None` until it has run.
pub fn status() -> GlassStatus {
    STATUS.get().copied().unwrap_or(GlassStatus::None)
}
