use serde::{Deserialize, Serialize};

use crate::models::MessagePart;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Agent,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentMessage {
    #[serde(rename = "messageId")]
    pub message_id: String,
    #[serde(rename = "contextId", skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub role: MessageRole,
    #[serde(default)]
    pub parts: Vec<MessagePart>,
}

impl AgentMessage {
    pub fn text(message_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            message_id: message_id.into(),
            context_id: None,
            task_id: None,
            role: MessageRole::User,
            parts: vec![MessagePart::Text { text: text.into() }],
        }
    }
}
