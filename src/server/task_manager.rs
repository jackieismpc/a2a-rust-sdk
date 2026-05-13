use std::sync::Arc;

use crate::core::{A2aError, ErrorCode, Result};
use crate::models::{A2aResponse, AgentCard, AgentMessage, AgentTask, AgentTaskStatus, MessageSendParams, TaskState};
use crate::server::{MemoryTaskStore, TaskStore};

pub type MessageCallback = Arc<dyn Fn(MessageSendParams) -> Result<A2aResponse> + Send + Sync>;
pub type TaskCallback = Arc<dyn Fn(AgentTask) + Send + Sync>;
pub type AgentCardCallback = Arc<dyn Fn(&str) -> AgentCard + Send + Sync>;

pub struct TaskManager {
    task_store: Arc<dyn TaskStore>,
    on_message_received: Option<MessageCallback>,
    on_task_created: Option<TaskCallback>,
    on_task_cancelled: Option<TaskCallback>,
    on_task_updated: Option<TaskCallback>,
    on_agent_card_query: Option<AgentCardCallback>,
    agent_card: Option<AgentCard>,
}

impl TaskManager {
    pub fn new(task_store: Option<Arc<dyn TaskStore>>) -> Self {
        let store = task_store.unwrap_or_else(|| Arc::new(MemoryTaskStore::new()));
        Self {
            task_store: store,
            on_message_received: None,
            on_task_created: None,
            on_task_cancelled: None,
            on_task_updated: None,
            on_agent_card_query: None,
            agent_card: None,
        }
    }

    pub fn set_on_message_received(&mut self, callback: MessageCallback) {
        self.on_message_received = Some(callback);
    }

    pub fn set_on_task_created(&mut self, callback: TaskCallback) {
        self.on_task_created = Some(callback);
    }

    pub fn set_on_task_cancelled(&mut self, callback: TaskCallback) {
        self.on_task_cancelled = Some(callback);
    }

    pub fn set_on_task_updated(&mut self, callback: TaskCallback) {
        self.on_task_updated = Some(callback);
    }

    pub fn set_on_agent_card_query(&mut self, callback: AgentCardCallback) {
        self.on_agent_card_query = Some(callback);
    }

    pub fn set_agent_card(&mut self, card: AgentCard) {
        self.agent_card = Some(card);
    }

    pub fn create_task(&self, context_id: Option<String>, task_id: Option<String>) -> AgentTask {
        let context_id = context_id.unwrap_or_else(|| format!("ctx-{}", uuid::Uuid::new_v4()));
        let task_id = task_id.unwrap_or_else(|| format!("task-{}", uuid::Uuid::new_v4()));
        let task = AgentTask {
            id: task_id.clone(),
            context_id,
            status: AgentTaskStatus::new(TaskState::Submitted),
            artifacts: Vec::new(),
            history: Vec::new(),
            metadata: Default::default(),
        };
        self.task_store.set_task(task.clone());
        if let Some(callback) = &self.on_task_created {
            callback(task.clone());
        }
        task
    }

    pub fn get_task(&self, task_id: &str) -> Result<AgentTask> {
        self.task_store
            .get_task(task_id)
            .ok_or_else(|| A2aError::from_code(ErrorCode::TaskNotFound, "task not found"))
    }

    pub fn cancel_task(&self, task_id: &str) -> Result<AgentTask> {
        let task = self.get_task(task_id)?;
        if task.status.is_terminal() {
            return Err(A2aError::from_code(
                ErrorCode::TaskNotCancelable,
                "task is terminal",
            ));
        }
        self.task_store
            .update_status(task_id, TaskState::Canceled, None);
        let updated = self.get_task(task_id)?;
        if let Some(callback) = &self.on_task_cancelled {
            callback(updated.clone());
        }
        Ok(updated)
    }

    pub fn update_status(&self, task_id: &str, status: TaskState, message: Option<&AgentMessage>) {
        if let Some(message) = message {
            self.task_store
                .add_history_message(task_id, message.clone());
        }
        self.task_store
            .update_status(task_id, status, message.map(|m| serde_json::to_string(m).unwrap_or_default()));
        if let Ok(task) = self.get_task(task_id) {
            if let Some(callback) = &self.on_task_updated {
                callback(task);
            }
        }
    }

    pub fn return_artifact(&self, task_id: &str, artifact: crate::models::Artifact) {
        self.task_store.add_artifact(task_id, artifact);
        if let Ok(task) = self.get_task(task_id) {
            if let Some(callback) = &self.on_task_updated {
                callback(task);
            }
        }
    }

    pub fn send_message(&self, params: MessageSendParams) -> Result<A2aResponse> {
        if let Some(task_id) = params.message.task_id.as_deref() {
            if !self.task_store.task_exists(task_id) {
                return Err(A2aError::from_code(ErrorCode::TaskNotFound, "task not found"));
            }
            self.task_store
                .add_history_message(task_id, params.message.clone());
        }

        let callback = self
            .on_message_received
            .as_ref()
            .ok_or_else(|| A2aError::from_code(ErrorCode::InternalError, "callback not set"))?;
        callback(params)
    }

    pub fn get_agent_card(&self, agent_url: &str) -> AgentCard {
        if let Some(card) = &self.agent_card {
            return card.clone();
        }

        self.on_agent_card_query
            .as_ref()
            .map(|callback| callback(agent_url))
            .unwrap_or_else(|| AgentCard::new("Unknown Agent", agent_url))
    }

    pub fn task_store(&self) -> Arc<dyn TaskStore> {
        self.task_store.clone()
    }
}
