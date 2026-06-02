use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no session found")]
    NoSession,

    #[error("session expired or invalid")]
    InvalidSession,

    #[error("logged in, but {0} is not visible to this session")]
    NoListAccess(String),

    #[error("email not found: {0}")]
    EmailNotFound(String),

    #[error("parent not found in archive")]
    ParentNotFound { in_reply_to: String },

    #[error("invalid mailing list address: {0}")]
    InvalidListAddress(String),

    #[error("invalid cookie input: {0}")]
    InvalidCookie(String),

    #[error("could not determine a configuration directory")]
    ConfigDirUnavailable,

    #[error("Pony Mail API response changed while reading {endpoint}: {reason}")]
    ApiShapeChanged {
        endpoint: &'static str,
        reason: String,
    },

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("URL construction failed: {0}")]
    Url(#[from] url::ParseError),

    #[error("keyring operation failed: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("I/O failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON operation failed: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
