use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::models::{AgentMessage, AgentTask, Artifact, TaskState};
use crate::server::TaskStore;

#[derive(Default, Clone)]
pub struct MemoryTaskStore {
    inner: Arc<Mutex<HashMap<String, AgentTask>>>,
}

impl MemoryTaskStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn size(&self) -> usize {
        self.inner.lock().expect("store lock").len()
    }

    pub fn clear(&self) {
        self.inner.lock().expect("store lock").clear();
    }
}

impl TaskStore for MemoryTaskStore {
    fn get_task(&self, task_id: &str) -> Option<AgentTask> {
        self.inner.lock().expect("store lock").get(task_id).cloned()
    }

    fn set_task(&self, task: AgentTask) {
        self.inner
            .lock()
            .expect("store lock")
            .insert(task.id.clone(), task);
    }

    fn update_status(&self, task_id: &str, status: TaskState, message: Option<String>) {
        if let Some(task) = self.inner.lock().expect("store lock").get_mut(task_id) {
            task.status.state = status;
            task.status.message = message;
        }
    }

    fn add_artifact(&self, task_id: &str, artifact: Artifact) {
        if let Some(task) = self.inner.lock().expect("store lock").get_mut(task_id) {
            task.artifacts.push(artifact);
        }
    }

    fn add_history_message(&self, task_id: &str, message: AgentMessage) {
        if let Some(task) = self.inner.lock().expect("store lock").get_mut(task_id) {
            task.history.push(message);
        }
    }

    fn get_history(&self, context_id: &str, max_length: usize) -> Vec<AgentMessage> {
        let store = self.inner.lock().expect("store lock");
        let mut history = Vec::new();
        for task in store.values() {
            if task.context_id == context_id {
                history.extend(task.history.clone());
            }
        }
        if max_length == 0 || history.len() <= max_length {
            return history;
        }
        history.split_off(history.len() - max_length)
    }

    fn delete_task(&self, task_id: &str) -> bool {
        self.inner
            .lock()
            .expect("store lock")
            .remove(task_id)
            .is_some()
    }

    fn task_exists(&self, task_id: &str) -> bool {
        self.inner.lock().expect("store lock").contains_key(task_id)
    }
}
