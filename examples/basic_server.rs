use a2a_rust_sdk::models::{A2aResponse, MessageRole};
use a2a_rust_sdk::server::{axum_router, TaskManager};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let mut manager = TaskManager::new(None);

    manager.set_on_message_received(Arc::new(|params| {
        let mut reply = params.message;
        reply.role = MessageRole::Agent;
        Ok(A2aResponse::Message(reply))
    }));

    let app = axum_router(Arc::new(manager));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:5000")
        .await
        .expect("bind server");

    println!("A2A server listening on http://127.0.0.1:5000");
    axum::serve(listener, app).await.expect("serve A2A server");
}
