use a2a_rust_sdk::models::*;
use a2a_rust_sdk::server::{MemoryTaskStore, TaskManager};
use std::sync::Arc;

#[test]
fn task_manager_lifecycle() {
    let store = Arc::new(MemoryTaskStore::new());
    let manager = TaskManager::new(Some(store));
    let task = manager.create_task(None, None);

    assert_eq!(task.status.state, TaskState::Submitted);
    let fetched = manager.get_task(&task.id).expect("get task");
    assert_eq!(fetched.id, task.id);

    let canceled = manager.cancel_task(&task.id).expect("cancel task");
    assert_eq!(canceled.status.state, TaskState::Canceled);
}
