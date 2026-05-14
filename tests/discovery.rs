use a2a_rust_sdk::models::{AgentAuthentication, AgentCard};
use a2a_rust_sdk::server::TaskManager;
use a2a_rust_sdk::server::axum_router;
use axum::body::Body;
use axum::body::to_bytes;
use axum::http::Method;
use axum::http::Request;
use axum::http::StatusCode;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn serve_agent_card() {
    let mut manager = TaskManager::new(None);
    let mut card = AgentCard::new("TestAgent", "http://127.0.0.1:5000");
    card.authentication = Some(AgentAuthentication {
        schemes: vec!["Bearer".to_string()],
        credentials: Some("secret-token".to_string()),
    });
    manager.set_agent_card(card);
    let app = axum_router(Arc::new(manager));

    let req = Request::builder()
        .method(Method::GET)
        .uri("/.well-known/agent-card.json")
        .header("host", "127.0.0.1:5000")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body(), usize::MAX).await.expect("body");
    let text = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(!text.contains("secret-token"));
}
