use serde::{Deserialize, Serialize};

use crate::models::{AgentMessage, AgentTask};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum A2aResponse {
    Task(AgentTask),
    Message(AgentMessage),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    Chunk(String),
}
