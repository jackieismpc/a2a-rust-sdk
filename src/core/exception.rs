use crate::core::ErrorCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum A2aError {
    #[error("http error: {0}")]
    Http(String),
    #[error("json error: {0}")]
    Json(String),
    #[error("rpc error: {code} {message}")]
    Rpc { code: i32, message: String, data: Option<String> },
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("task error: {0}")]
    Task(String),
    #[error("timeout")]
    Timeout,
}

pub type Result<T> = std::result::Result<T, A2aError>;

impl From<serde_json::Error> for A2aError {
    fn from(err: serde_json::Error) -> Self {
        A2aError::Json(err.to_string())
    }
}

impl From<reqwest::Error> for A2aError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            return A2aError::Timeout;
        }
        A2aError::Http(err.to_string())
    }
}

impl A2aError {
    pub fn from_code(code: ErrorCode, message: impl Into<String>) -> Self {
        A2aError::Rpc {
            code: code.as_i32(),
            message: message.into(),
            data: None,
        }
    }
}
