use a2a_rust_sdk::models::AgentCard;
use a2a_rust_sdk::server::TaskManager;
use a2a_rust_sdk::server::axum_router;
use axum::body::Body;
use axum::http::Method;
use axum::http::Request;
use axum::http::StatusCode;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn serve_agent_card() {
    let mut manager = TaskManager::new(None);
    manager.set_agent_card(AgentCard::new("TestAgent", "http://127.0.0.1:5000"));
    let app = axum_router(Arc::new(manager));

    let req = Request::builder()
        .method(Method::GET)
        .uri("/.well-known/agent-card.json")
        .header("host", "127.0.0.1:5000")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
}
