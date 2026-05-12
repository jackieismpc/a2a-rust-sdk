use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::models::{AgentMessage, AgentTaskStatus, Artifact};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentTask {
    pub id: String,
    #[serde(rename = "contextId")]
    pub context_id: String,
    pub status: AgentTaskStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<AgentMessage>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}
