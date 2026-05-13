use a2a_rust_sdk::client::A2aClient;
use a2a_rust_sdk::models::{AgentMessage, MessagePart, MessageRole, MessageSendParams};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = env::var("A2A_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:5000".to_string());
    let client = A2aClient::new(base_url);

    let params = MessageSendParams {
        message: AgentMessage {
            message_id: "msg-demo-1".to_string(),
            context_id: None,
            task_id: None,
            role: MessageRole::User,
            parts: vec![MessagePart::Text {
                text: "你好，帮我做一个最小 A2A 测试".to_string(),
            }],
        },
        history_length: None,
        context_id: None,
        task_id: None,
    };

    match client.send_message(params).await {
        Ok(response) => println!("response: {response:?}"),
        Err(error) => eprintln!("request failed: {error}"),
    }

    Ok(())
}
