use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::models::AgentMessage;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageSendParams {
    pub message: AgentMessage,
    #[serde(rename = "historyLength", skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,
    #[serde(rename = "contextId", skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskQueryParams {
    pub id: String,
    #[serde(rename = "historyLength", skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskIdParams {
    pub id: String,
}
