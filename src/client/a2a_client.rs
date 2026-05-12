use bytes::Bytes;
use reqwest::Client;
use serde_json::Value;

use crate::core::{A2aError, ErrorCode, JsonRpcRequest, JsonRpcResponse, Result};
use crate::models::{A2aResponse, MessageSendParams, StreamEvent, TaskIdParams};

pub struct A2aClient {
    base_url: String,
    client: Client,
}

impl A2aClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let mut base_url = base_url.into();
        if base_url.ends_with('/') {
            base_url.pop();
        }
        let client = Client::builder().build().expect("reqwest client");
        Self { base_url, client }
    }

    pub async fn send_message(&self, params: MessageSendParams) -> Result<A2aResponse> {
        let request = JsonRpcRequest::new(Value::String(self.request_id()), "message/send", Some(serde_json::to_value(params)?));
        let response = self.post_json(request).await?;
        self.parse_response(response)
    }

    pub async fn send_message_streaming<F>(&self, params: MessageSendParams, mut on_event: F) -> Result<()>
    where
        F: FnMut(StreamEvent),
    {
        let request = JsonRpcRequest::new(Value::String(self.request_id()), "message/stream", Some(serde_json::to_value(params)?));
        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(A2aError::Http(response.status().to_string()));
        }

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                on_event(StreamEvent::Chunk(text));
            }
        }

        Ok(())
    }

    pub async fn get_task(&self, task_id: &str) -> Result<crate::models::AgentTask> {
        let params = TaskIdParams { id: task_id.to_string() };
        let request = JsonRpcRequest::new(Value::String(self.request_id()), "tasks/get", Some(serde_json::to_value(params)?));
        let response = self.post_json(request).await?;
        let value = self.expect_result(response)?;
        Ok(serde_json::from_value(value)?)
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<crate::models::AgentTask> {
        let params = TaskIdParams { id: task_id.to_string() };
        let request = JsonRpcRequest::new(Value::String(self.request_id()), "tasks/cancel", Some(serde_json::to_value(params)?));
        let response = self.post_json(request).await?;
        let value = self.expect_result(response)?;
        Ok(serde_json::from_value(value)?)
    }

    pub async fn subscribe_to_task<F>(&self, task_id: &str, on_event: F) -> Result<()>
    where
        F: FnMut(StreamEvent),
    {
        let params = TaskIdParams { id: task_id.to_string() };
        let request = JsonRpcRequest::new(Value::String(self.request_id()), "tasks/resubscribe", Some(serde_json::to_value(params)?));
        self.send_stream_request(request, on_event).await
    }

    pub fn set_timeout(&self, _timeout: std::time::Duration) {
        // reqwest client timeout is configured at build time; use builder if needed.
    }

    fn request_id(&self) -> String {
        format!("req-{}", uuid::Uuid::new_v4())
    }

    async fn send_stream_request<F>(&self, request: JsonRpcRequest, mut on_event: F) -> Result<()>
    where
        F: FnMut(StreamEvent),
    {
        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(A2aError::Http(response.status().to_string()));
        }

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            on_event(StreamEvent::Chunk(as_string(chunk)));
        }

        Ok(())
    }

    async fn post_json(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(A2aError::Http(response.status().to_string()));
        }

        let body = response.json::<JsonRpcResponse>().await?;
        Ok(body)
    }

    fn parse_response(&self, response: JsonRpcResponse) -> Result<A2aResponse> {
        let value = self.expect_result(response)?;
        if value.get("status").is_some() {
            Ok(A2aResponse::Task(serde_json::from_value(value)?))
        } else {
            Ok(A2aResponse::Message(serde_json::from_value(value)?))
        }
    }

    fn expect_result(&self, response: JsonRpcResponse) -> Result<Value> {
        if let Some(error) = response.error {
            return Err(A2aError::Rpc {
                code: error.code,
                message: error.message,
                data: error.data.map(|d| d.to_string()),
            });
        }

        response
            .result
            .ok_or_else(|| A2aError::from_code(ErrorCode::InternalError, "missing result"))
    }
}

fn as_string(bytes: Bytes) -> String {
    String::from_utf8(bytes.to_vec()).unwrap_or_default()
}

use futures_util::StreamExt;
