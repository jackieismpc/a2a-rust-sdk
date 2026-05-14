use serde::{Deserialize, Serialize};

/// Defines error codes for JSON-RPC responses, including standard and custom application errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum ErrorCode {
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    TaskNotFound = -32001,
    TaskNotCancelable = -32002,
    UnsupportedOperation = -32003,
    ContentTypeNotSupported = -32004,
    PushNotificationNotSupported = -32005,
}

impl ErrorCode {
    /// Converts the `ErrorCode` enum variant to its corresponding integer code.
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}
