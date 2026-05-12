# A2A Rust SDK 开发文档

本文档描述 A2A (Agent-to-Agent) 协议的 Rust 实现方案。目标是构建更 Rustc 风格、更 Rust 原生、更易于集成到 Rust 服务栈（例如 axum）的版本。

## 目标

- 提供以 Rust 生态为核心的生产级 SDK，避免不必要的 C/C++ 依赖。
- 保持协议兼容性，但允许在可扩展性、错误处理、类型系统与 API 体验上进行优化。
- 面向 axum/async 生态提供友好的服务端 API 与中间件式集成方式。

## 协议概要

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

Rust 结构体使用 snake_case，但通过 serde rename 保持 JSON 字段名不变。

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

## 规范对照与实现状态（基于官网）

参考来源：

- https://a2acn.com/specification/core/
- https://a2acn.com/specification/discovery/
- https://a2acn.com/docs/concepts/agentcard/
- https://a2acn.com/docs/concepts/task/
- https://a2acn.com/docs/topics/streaming-and-async/

### 字段级对照（精细版）

以下对照仅基于官网当前公开文档所描述的字段与语义，不包含仓库内部 proto 的完整细节。

#### AgentCard

| 规范字段/行为 | 规范描述 | 当前实现 | 差距/备注 |
| --- | --- | --- | --- |
| `name`/`description`/`url`/`version` | 基本信息 | 已实现 | OK |
| `documentationUrl` | 文档链接 | 已实现 | OK |
| `provider.organization/url` | 提供方信息 | 部分实现（无 `url` 必填约束） | 需补齐结构与校验 |
| `capabilities.streaming` | 是否支持 SSE | 已实现 | OK |
| `capabilities.pushNotifications` | 是否支持推送 | 已实现 | OK |
| `capabilities.stateTransitionHistory` | 状态变更历史能力 | 未实现 | 需补字段与语义 |
| `authentication.schemes/credentials` | 认证要求 | 未实现 | 需引入认证模型 |
| `defaultInputModes/defaultOutputModes` | 默认 MIME 模式 | 已实现 | OK |
| `skills[].id/tags/examples` | 技能元数据 | 未实现 | 需补字段 |
| `skills[].inputModes/outputModes` | 技能级 MIME 模式 | 已实现 | OK |
| `/.well-known/agent-card.json` | 发现路径 | 未实现 | 需增加路由与访问控制 |

#### Task / TaskStatus

| 规范字段/行为 | 规范描述 | 当前实现 | 差距/备注 |
| --- | --- | --- | --- |
| `id` | 任务标识 | 已实现 | v1.0 语义要求服务端生成 |
| `sessionId` | 会话标识 | 未实现（使用 `contextId`） | 需对齐命名与语义 |
| `status.state` | 状态枚举 | 部分实现 | 缺 `working/input-required/unknown` |
| `status.message` | 状态消息（Message） | 部分实现（字符串） | 需改为 Message |
| `status.timestamp` | 时间戳 | 已实现 | OK |
| `history` | 消息历史 | 已实现 | 需按规范结构校验 |
| `artifacts` | 工件集合 | 已实现 | OK |
| `metadata` | 扩展元数据 | 已实现（字符串 map） | 规范为任意 JSON |

#### Message

| 规范字段/行为 | 规范描述 | 当前实现 | 差距/备注 |
| --- | --- | --- | --- |
| `role` | `user` / `agent` | 已实现（额外支持 `system`） | 需明确兼容策略 |
| `parts` | Part 数组 | 已实现 | OK |
| `metadata` | 扩展元数据 | 未实现 | 需补字段 |

#### Part

| 规范字段/行为 | 规范描述 | 当前实现 | 差距/备注 |
| --- | --- | --- | --- |
| `type` (MIME) | MIME/类型标识 | 未实现（使用 `kind`） | 需改为 `type` 或双写兼容 |
| TextPart | 文本 | 已实现 | 需按 MIME 规范化 |
| FilePart | 文件 URI/Base64/filename | 部分实现（Base64 + filename） | 缺 URI/metadata 支持 |
| JsonPart | 结构化 JSON | 部分实现（Data） | 字段名需对齐 |
| Form/IFrame/扩展 | 复杂交互片段 | 未实现 | 需扩展 |

#### Streaming / Async

| 规范字段/行为 | 规范描述 | 当前实现 | 差距/备注 |
| --- | --- | --- | --- |
| `message/stream` | SSE 流 | 已实现 | 事件结构需对齐 |
| `TaskStatusUpdateEvent` | 状态更新事件 | 未实现 | 需实现事件类型 |
| `TaskArtifactUpdateEvent` | 工件更新事件 | 未实现 | 需实现事件类型 |
| 结束语义 | 以状态/流关闭判断 | 部分实现 | 当前 start/result/done 为自定义 |
| 推送通知 | 断线回退机制 | 未实现 | 需配合 push config |

#### 发现与安全

| 规范字段/行为 | 规范描述 | 当前实现 | 差距/备注 |
| --- | --- | --- | --- |
| `.well-known/agent-card.json` | 开放发现 | 未实现 | 需路由与访问控制 |
| 认证与授权 | OAuth/mTLS 等 | 未实现 | 需加入认证策略 |
| 安全实践 | 最小权限、审计 | 未实现 | 需指南与实现 |

#### 版本与扩展

| 规范字段/行为 | 规范描述 | 当前实现 | 差距/备注 |
| --- | --- | --- | --- |
| `A2A-Version` | 版本头 | 未实现 | 需加入请求/响应处理 |
| `A2A-Extensions` | 扩展声明 | 未实现 | 需扩展体系 |

### 已实现或基本可用

- JSON-RPC over HTTP 基础请求/响应
- `message/send`、`message/stream`、`tasks/get`、`tasks/cancel`、`tasks/resubscribe`
- 基础数据模型：`AgentCard`、`AgentTask`、`AgentMessage`、`Artifact`
- SSE 流式基础通路（start/result/done 事件）

### 部分实现（需补齐细节）

- Task 状态：当前仅 `submitted/running/completed/failed/canceled/rejected`，规范还包含 `working/input-required/unknown`。
- TaskStatus：规范中的 `message` 应为 `Message`，当前是字符串。
- Task 结构：规范包含 `sessionId`、更多 metadata 扩展字段，当前缺失或简化。
- Message：规范支持 metadata，当前未实现。
- Part：规范以 `type` (MIME) 为核心，支持 text/json/file/form/iframe 等，当前仅 `text/file/data` 且字段名为 `kind`。
- AgentCard：规范包含 `authentication`、技能 `id/tags/examples`、`capabilities.stateTransitionHistory`，当前未覆盖。
- Streaming：规范事件类型包含 `TaskStatusUpdateEvent/TaskArtifactUpdateEvent`，当前 SSE 仅输出自定义 start/result/done。

### 未实现或缺失

- 发现机制：`.well-known/agent-card.json` 端点与安全控制策略
- 推送通知能力与配置（pushNotificationConfig 相关操作）
- `ListTasks`、`GetExtendedAgentCard` 等核心协议操作
- 版本与扩展头：`A2A-Version`、`A2A-Extensions`
- v1.0 语义：流结束不依赖 `final` 字段；任务 ID 由服务端生成
- 认证与授权的标准化实践（security guide 相关要求）
- gRPC 绑定（规范支持 JSON-RPC/HTTP + gRPC + REST 绑定）

## 下一阶段改进路线

### Phase 1: 协议对齐与模型完善

- 对齐 TaskState（增加 `working/input-required/unknown`）
- TaskStatus.message 改为完整 Message 结构
- Message/Part 增加 metadata 与 MIME type 模式
- AgentCard 增加 authentication、技能 id/tags/examples、stateTransitionHistory
- 实现 `.well-known/agent-card.json`

### Phase 2: 传输与事件语义对齐

- SSE 事件结构对齐：`TaskStatusUpdateEvent` / `TaskArtifactUpdateEvent`
- 加入 pushNotificationConfig 及相关操作
- 追加 `ListTasks` / `GetExtendedAgentCard`
- 支持 `A2A-Version` 与 `A2A-Extensions` 头

### Phase 3: 绑定与企业就绪能力

- 引入 gRPC 绑定（基于官方 proto 生成）
- 安全策略实现：OAuth/mTLS/签名校验
- 扩展机制的 schema 版本化与兼容性测试

## 性能分析与改进方案

### 现状

- JSON-RPC + HTTP + SSE 实现易调试、生态友好，但序列化与带宽开销高于 protobuf。
- SSE 为单向流，长任务场景需配合 push/轮询，效率较低。

### 改进方向

- 引入 gRPC 绑定：protobuf + HTTP/2 + 双向流，提升吞吐与延迟表现。
- 统一使用官方 proto 生成模型，减少手工维护与兼容风险。
- 客户端与服务端使用连接池、HTTP/2、多路复用。
- 结构化日志与 tracing，定位序列化与网络瓶颈。
- 使用压缩与零拷贝（bytes），降低大消息成本。

### 迁移建议

- 对外保持 JSON-RPC 接口，对内引入 gRPC
- 新增 feature flag 选择 JSON 或 gRPC 绑定
- 以性能基准作为 gate：JSON 与 gRPC 统一测评后再切换默认
