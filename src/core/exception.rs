use crate::core::ErrorCode;
use thiserror::Error;

/// Errors that can occur while interacting with the A2A protocol.
///
/// This type represents transport-level, serialization,
/// protocol, and task execution failures returned by the SDK.
#[derive(Debug, Error)]
pub enum A2aError {
    /// HTTP transport error.
    #[error("http error: {0}")]
    Http(String),

    /// JSON serialization or deserialization failure.
    #[error("json error: {0}")]
    Json(String),

    /// JSON-RPC error returned by the remote peer.
    #[error("rpc error: {code} {message}")]
    Rpc {
        code: i32,
        message: String,
        data: Option<String>,
    },

    /// Protocol-level validation or format error.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// Task execution or lifecycle error.
    #[error("task error: {0}")]
    Task(String),

    /// Request timed out.
    #[error("timeout")]
    Timeout,
}

/// Alias for results returned by A2A SDK methods.
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
    /// Creates a new `A2aError::Rpc` with the given error code and message.
    pub fn from_code(code: ErrorCode, message: impl Into<String>) -> Self {
        A2aError::Rpc {
            code: code.as_i32(),
            message: message.into(),
            data: None,
        }
    }
    /// Creates a new `A2aError::Rpc` with the given error code, message, and optional data.
    pub fn from_code_with_data(
        code: ErrorCode,
        message: impl Into<String>,
        data: Option<String>,
    ) -> Self {
        A2aError::Rpc {
            code: code.as_i32(),
            message: message.into(),
            data,
        }
    }
}
