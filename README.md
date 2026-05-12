# A2A Rust SDK 开发文档

本文档描述 A2A (Agent-to-Agent) 协议的 Rust 实现方案。它以本工作区中的 C++ SDK 为参考，但目标是构建更 Rustc 风格、更 Rust 原生、更易于集成到 Rust 服务栈（例如 axum）的版本，而不是 1:1 复刻。

## 目标

- 提供以 Rust 生态为核心的生产级 SDK，避免不必要的 C/C++ 依赖。
- 保持协议兼容性，但允许在可扩展性、错误处理、类型系统与 API 体验上进行优化。
- 面向 axum/async 生态提供友好的服务端 API 与中间件式集成方式。

## 协议概要（基于现有实现）

### JSON-RPC 2.0

请求：

```json
{
  "jsonrpc": "2.0",
  "id": "req-1-1710000000",
  "method": "message/send",
  "params": { ... }
}
```

响应（成功）：

```json
{
  "jsonrpc": "2.0",
  "id": "req-1-1710000000",
  "result": { ... }
}
```

响应（错误）：

```json
{
  "jsonrpc": "2.0",
  "id": "req-1-1710000000",
  "error": {
    "code": -32602,
    "message": "Invalid params",
    "data": "optional"
  }
}
```

### 方法

- `message/send`（非流式）
- `message/stream`（流式）
- `tasks/get`
- `tasks/cancel`
- `tasks/resubscribe`（流式）
- `tasks/pushNotificationConfig/set`
- `tasks/pushNotificationConfig/get`

### 错误码

- JSON-RPC 标准：`ParseError -32700`、`InvalidRequest -32600`、`MethodNotFound -32601`、`InvalidParams -32602`、`InternalError -32603`
- A2A：`TaskNotFound -32001`、`TaskNotCancelable -32002`、`UnsupportedOperation -32003`、`ContentTypeNotSupported -32004`、`PushNotificationNotSupported -32005`

## 数据模型与 JSON

以下字段名与 C++ 序列化逻辑一致。Rust 结构体使用 snake_case，但通过 serde rename 保持 JSON 字段名不变。

### AgentCard

```json
{
  "name": "Math Agent",
  "description": "Solves math",
  "url": "http://localhost:5001",
  "version": "1.0.0",
  "protocolVersion": "0.3.0",
  "capabilities": {
    "streaming": true,
    "pushNotifications": false,
    "taskManagement": true
  },
  "defaultInputModes": ["text"],
  "defaultOutputModes": ["text"],
  "skills": [
    {
      "name": "arithmetic",
      "description": "basic math",
      "inputModes": ["text"],
      "outputModes": ["text"]
    }
  ],
  "preferredTransport": "jsonrpc",
  "iconUrl": "optional",
  "documentationUrl": "optional",
  "provider": {
    "name": "Provider",
    "organization": "Org",
    "url": "optional"
  }
}
```

### AgentMessage

```json
{
  "messageId": "msg-1710000000",
  "contextId": "ctx-1-1710000000",
  "taskId": "task-1-1710000000",
  "role": "user",
  "parts": [
    { "kind": "text", "text": "hello" },
    {
      "kind": "file",
      "file": {
        "filename": "image.png",
        "mimeType": "image/png",
        "data": "BASE64"
      }
    },
    { "kind": "data", "data": { "key": "value" } }
  ]
}
```

角色：`user`、`agent`、`system`。

### MessageSendParams

```json
{
  "message": { ...AgentMessage... },
  "historyLength": 6,
  "contextId": "ctx-1-1710000000",
  "taskId": "task-1-1710000000"
}
```

### AgentTask 与 Status

```json
{
  "id": "task-1-1710000000",
  "contextId": "ctx-1-1710000000",
  "status": {
    "state": "running",
    "timestamp": "2024-01-01T00:00:00.000Z",
    "message": "optional"
  },
  "artifacts": [
    {
      "id": "artifact-1",
      "name": "result",
      "description": "optional",
      "mimeType": "text/plain",
      "url": "optional",
      "content": "optional",
      "metadata": { "key": "value" }
    }
  ],
  "history": [ ...AgentMessage... ],
  "metadata": { "key": "value" }
}
```

任务状态：`submitted`、`running`、`completed`、`failed`、`canceled`、`rejected`。

## 传输行为

- 使用 HTTP POST 向 Agent 基础 URL 发送请求，`Content-Type: application/json`。
- 流式方法设置 `Accept: text/event-stream`，以 SSE 方式输出事件。
- Rust SDK 仍保持与协议兼容，但错误映射更细致（详见错误处理部分）。

## Rust API 设计（建议）

### Crate 模块

```
crate::core
  - jsonrpc_request, jsonrpc_response, error_code, exception
crate::models
  - agent_card, agent_message, message_part, agent_task, artifact, task_status
crate::client
  - a2a_client
crate::server
  - task_manager, task_store, memory_task_store
```

### 客户端 API

- `A2aClient::new(base_url: impl Into<String>)`
- `send_message(params: MessageSendParams) -> Result<A2aResponse>`
- `send_message_streaming(params, on_event: impl FnMut(StreamEvent)) -> Result<()>`
- `get_task(task_id: &str) -> Result<AgentTask>`
- `cancel_task(task_id: &str) -> Result<AgentTask>`
- `subscribe_to_task(task_id: &str, on_event: impl FnMut(StreamEvent)) -> Result<()>`
- `set_timeout(Duration)`

### 服务端 API（axum 友好）

- `TaskManager::new(store: Arc<dyn TaskStore>)`
- `TaskManager::with_router(self, Router) -> Router`（挂载 JSON-RPC 路由）
- `TaskManager::with_sse(self, Router) -> Router`（挂载 SSE 流式路由）
- `set_on_message_received(fn(MessageSendParams) -> A2aResponse)`
- `set_on_task_created(fn(AgentTask))`
- `set_on_task_cancelled(fn(AgentTask))`
- `set_on_task_updated(fn(AgentTask))`
- `set_on_agent_card_query(fn(agent_url: &str) -> AgentCard)`
- `create_task(context_id: Option<String>, task_id: Option<String>) -> AgentTask`
- `get_task(task_id: &str) -> Result<AgentTask>`
- `cancel_task(task_id: &str) -> Result<AgentTask>`
- `update_status(task_id: &str, state: TaskState, message: Option<&AgentMessage>)`
- `return_artifact(task_id: &str, artifact: Artifact)`

## axum 集成示例

```rust
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
  let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await.unwrap();
  axum::serve(listener, app).await.unwrap();
}
```

### TaskStore Trait

- `get_task(task_id) -> Option<AgentTask>`
- `set_task(task)`
- `update_status(task_id, state, message)`
- `add_artifact(task_id, artifact)`
- `add_history_message(task_id, message)`
- `get_history(context_id, max_length) -> Vec<AgentMessage>`
- `delete_task(task_id) -> bool`
- `task_exists(task_id) -> bool`

## 序列化说明

- 所有 JSON 处理使用 `serde` + `serde_json`。
- 文件 part 的字节使用 `base64` crate 进行编码。
- 任务状态时间戳使用带毫秒的 ISO 8601 格式并以 `Z` 结尾。
- JSON 字段名必须与示例保持一致。

## 建议依赖

- `serde`, `serde_json`
- `tokio` for async runtime
- `hyper` + `hyper-util` or `reqwest`（纯 Rust TLS）
- `rustls` + `tokio-rustls`（替代 openssl）
- `axum`（服务端路由与提取器）
- `tower` / `tower-http`（中间件与超时/重试）
- `bytes`（流式数据）
- `base64`
- `time` 或 `chrono`（时间戳）
- `thiserror`（错误类型）

## 测试清单

- 所有模型的 JSON 循环序列化
- JSON-RPC 请求/响应解析（包含数值类型 `id`）
- SSE 风格分块的消息流
- 任务生命周期：创建、更新、取消、终态
- `TaskStore::get_history` 的历史长度裁剪
- axum 路由集成与提取器兼容性测试
- TLS 连接与证书加载（rustls 路径）

## 兼容性说明

- C++ 客户端直接向基础 URL 发送 POST，请求路径不追加额外路由。
- 流式响应以原始分块传递，SDK 应原样透传。
- AgentCard 的 `protocolVersion` 默认值为 `0.3.0`。

## Rust 优先设计原则

- Rust 原生 TLS（rustls），不依赖 openssl。
- 错误类型使用 `thiserror`，提供清晰的错误层级与可序列化错误体。
- 对外 API 尽量无锁、零拷贝友好，并与 `Send + Sync` 约束一致。
- 服务端优先考虑 axum 的 extractor、router 与 middleware 集成。

## 错误处理策略（建议）

- JSON-RPC 错误码保持兼容。
- SDK 内部错误以 Rust 错误类型区分：网络错误、序列化错误、协议错误、业务错误。
- HTTP 状态与 JSON-RPC 错误码分别处理，保留更多上下文信息。
