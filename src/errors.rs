/// CAUS-INTERNAL-51, CAUS-INTERNAL-55:
/// Regeneration error model for user-facing CLI diagnostics.
#[derive(thiserror::Error, Debug)]
pub enum FerriteError {
    #[error("Connection failed: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Invalid request item '{item}': {reason}")]
    ParseError { item: String, reason: String },

    #[error("Session error: {0}")]
    Session(String),

    #[error("Download failed: {0}")]
    Download(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("HTTP {status}: {reason}")]
    HttpError { status: u16, reason: String },

    #[error("Timeout after {secs}s")]
    Timeout { secs: f64 },

    #[error("TLS error: {0}")]
    Tls(String),
}

impl From<reqwest::Error> for FerriteError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            FerriteError::Network(format!("request timed out: {e}"))
        } else if e.is_connect() {
            FerriteError::Network(format!("connection refused: {e}"))
        } else {
            FerriteError::Network(e.to_string())
        }
    }
}
