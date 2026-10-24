use serde::{Deserialize, Serialize};
use specta::Type;

/// The one error type at the command boundary; serialises to `{ code, message }`.
/// Kernel code returns `anyhow::Result` and raises an `AppError` where the code matters;
/// `From<anyhow::Error>` recovers it, and anything else becomes `internal`.
#[derive(Debug, Clone, Serialize, Deserialize, Type, thiserror::Error)]
#[error("{code}: {message}")]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid_input", message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast::<AppError>() {
            Ok(app_err) => app_err,
            Err(err) => {
                tracing::error!("{err:#}");
                Self::new("internal", format!("{err:#}"))
            }
        }
    }
}
