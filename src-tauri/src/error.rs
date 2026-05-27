//! Application error type shared across modules and returned to the frontend.
//!
//! Privacy note (PRD §13.5): error messages must never embed API keys,
//! translation text, prompts, clipboard contents, or provider responses.

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("secret storage error: {0}")]
    Secret(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("capture error: {0}")]
    Capture(String),
}

// Tauri requires command error types to be serializable. We expose only the
// human-readable message string to the frontend.
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
