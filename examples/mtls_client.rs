use a2a_rust_sdk::models::{AgentMessage, MessagePart, MessageRole, MessageSendParams};
use reqwest::Certificate;
use reqwest::Identity;
use serde_json::Value;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ca = fs::read("examples/certs/ca.pem")?;
    let cert = fs::read("examples/certs/client.pem")?;
    let key = fs::read("examples/certs/client.key")?;

    let mut pem = Vec::new();
    pem.extend_from_slice(&cert);
    pem.extend_from_slice(&key);

    let identity = Identity::from_pem(&pem)?;
    let ca_cert = Certificate::from_pem(&ca)?;

    let client = reqwest::Client::builder()
        .identity(identity)
        .add_root_certificate(ca_cert)
        .build()?;

    let params = MessageSendParams {
        message: AgentMessage {
            message_id: "msg-mtls-1".to_string(),
            context_id: None,
            task_id: None,
            role: MessageRole::User,
            parts: vec![MessagePart::Text {
                text: "hello mtls".to_string(),
            }],
        },
        history_length: None,
        context_id: None,
        task_id: None,
    };

    let request = a2a_rust_sdk::core::JsonRpcRequest::new(
        Value::String("req-1".to_string()),
        "message/send",
        Some(serde_json::to_value(params)?),
    );

    let response = client
        .post("https://127.0.0.1:5443")
        .header("Authorization", "Bearer demo-token")
        .json(&request)
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;
    println!("status: {}", status);
    println!("body: {}", body);

    Ok(())
}
