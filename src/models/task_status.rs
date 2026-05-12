use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Submitted,
    Running,
    Completed,
    Failed,
    Canceled,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentTaskStatus {
    pub state: TaskState,
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl AgentTaskStatus {
    pub fn new(state: TaskState) -> Self {
        Self {
            state,
            timestamp: OffsetDateTime::now_utc(),
            message: None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            TaskState::Completed | TaskState::Failed | TaskState::Canceled | TaskState::Rejected
        )
    }
}
