use a2a_rust_sdk::models::*;
use serde_json::json;

#[test]
fn agent_message_round_trip() {
    let message = AgentMessage {
        message_id: "msg-1".to_string(),
        context_id: Some("ctx-1".to_string()),
        task_id: Some("task-1".to_string()),
        role: MessageRole::User,
        parts: vec![
            MessagePart::Text {
                text: "hello".to_string(),
            },
            MessagePart::Data {
                data: json!({"key": "value"}),
            },
        ],
    };

    let json_text = serde_json::to_string(&message).expect("serialize");
    let decoded: AgentMessage = serde_json::from_str(&json_text).expect("deserialize");
    assert_eq!(decoded, message);
}

#[test]
fn agent_task_round_trip() {
    let task = AgentTask {
        id: "task-1".to_string(),
        context_id: "ctx-1".to_string(),
        status: AgentTaskStatus::new(TaskState::Running),
        artifacts: vec![Artifact {
            id: "artifact-1".to_string(),
            name: "result".to_string(),
            description: Some("desc".to_string()),
            mime_type: Some("text/plain".to_string()),
            url: None,
            content: None,
            metadata: Default::default(),
        }],
        history: vec![],
        metadata: Default::default(),
    };

    let json_text = serde_json::to_string(&task).expect("serialize");
    let decoded: AgentTask = serde_json::from_str(&json_text).expect("deserialize");
    assert_eq!(decoded.id, task.id);
    assert_eq!(decoded.context_id, task.context_id);
    assert_eq!(decoded.status.state, task.status.state);
    assert_eq!(decoded.artifacts.len(), 1);
}
