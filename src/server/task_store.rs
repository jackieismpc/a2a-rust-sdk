use crate::models::{AgentMessage, AgentTask, Artifact, TaskState};

pub trait TaskStore: Send + Sync {
    fn get_task(&self, task_id: &str) -> Option<AgentTask>;
    fn set_task(&self, task: AgentTask);
    fn update_status(&self, task_id: &str, status: TaskState, message: Option<String>);
    fn add_artifact(&self, task_id: &str, artifact: Artifact);
    fn add_history_message(&self, task_id: &str, message: AgentMessage);
    fn get_history(&self, context_id: &str, max_length: usize) -> Vec<AgentMessage>;
    fn delete_task(&self, task_id: &str) -> bool;
    fn task_exists(&self, task_id: &str) -> bool;
}
