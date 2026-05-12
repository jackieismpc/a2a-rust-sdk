use a2a_rust_sdk::models::{A2aResponse, AgentMessage, MessageRole, MessageSendParams, MessagePart};
use a2a_rust_sdk::server::{axum_router, TaskManager};
use axum::http::{Request, StatusCode};
use axum::body::Body;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn axum_message_send_ok() {
    let mut manager = TaskManager::new(None);
    manager.set_on_message_received(Arc::new(|params| {
        let mut message = params.message;
        message.role = MessageRole::Agent;
        Ok(A2aResponse::Message(message))
    }));

    let app = axum_router(Arc::new(manager));

    let payload = MessageSendParams {
        message: AgentMessage {
            message_id: "msg-1".to_string(),
            context_id: None,
            task_id: None,
            role: MessageRole::User,
            parts: vec![MessagePart::Text { text: "hi".to_string() }],
        },
        history_length: None,
        context_id: None,
        task_id: None,
    };

    let request = Request::builder()
        .method("POST")
        .uri("/")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "jsonrpc": "2.0",
                "id": "req-1",
                "method": "message/send",
                "params": payload,
            }))
            .expect("serialize"),
        ))
        .expect("request");

    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}
