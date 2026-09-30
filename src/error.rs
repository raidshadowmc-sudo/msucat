use thiserror::Error;

#[derive(Error, Debug)]
pub enum MsuCatError {
    #[error("HTTP network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("HTML parsing error: {0}")]
    Parse(String),

    #[error("Update '{0}' not found in Microsoft Update Catalog")]
    NotFound(String),

    #[error("Download error for '{file}': {reason}")]
    Download { file: String, reason: String },

    #[error("Checksum mismatch for '{file}': expected {expected}, calculated {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, MsuCatError>;
