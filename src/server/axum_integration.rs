use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::routing::get;
use axum::http::Request;
use axum::Json;
use axum::Router;
use serde_json::Value;
use tokio_stream::iter;

use crate::core::{A2aError, ErrorCode, JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::models::{A2aResponse, MessageSendParams, TaskIdParams};
use crate::server::TaskManager;

#[derive(Clone)]
pub struct AxumState {
    pub manager: Arc<TaskManager>,
}

pub fn axum_router(manager: Arc<TaskManager>) -> Router {
    let state = AxumState { manager };
    Router::new()
        .route("/", post(handle_rpc))
        .route("/stream", post(handle_stream))
        .route("/.well-known/agent-card.json", get(handle_agent_card))
        .with_state(state)
}

async fn handle_agent_card(State(state): State<AxumState>, req: Request<axum::body::Body>) -> impl IntoResponse {
    // Try to derive a base URL from Host header, fallback to localhost:5000
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("127.0.0.1:5000");

    let url = format!("http://{}", host);
    let card = state.manager.get_agent_card(&url);
    (StatusCode::OK, axum::Json(card))
}

async fn handle_rpc(State(state): State<AxumState>, Json(request): Json<JsonRpcRequest>) -> impl IntoResponse {
    let result = match request.method.as_str() {
        "message/send" => handle_message_send(&state, request.params),
        "tasks/get" => handle_task_get(&state, request.params),
        "tasks/cancel" => handle_task_cancel(&state, request.params),
        _ => Err(A2aError::from_code(ErrorCode::MethodNotFound, "method not found")),
    };

    let response = match result {
        Ok(value) => JsonRpcResponse::success(request.id, value),
        Err(error) => JsonRpcResponse::error(request.id, to_jsonrpc_error(error)),
    };

    (StatusCode::OK, Json(response))
}

async fn handle_stream(
    State(state): State<AxumState>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let result = match request.method.as_str() {
        "message/stream" => handle_message_stream(&state, request.params).await,
        "tasks/resubscribe" => handle_task_resubscribe(&state, request.params),
        _ => Err(A2aError::from_code(ErrorCode::MethodNotFound, "method not found")),
    };

    match result {
        Ok(payloads) => {
            let events = iter(payloads.into_iter().map(|payload| {
                Ok::<Event, Infallible>(Event::default().data(payload))
            }));
            Sse::new(events).into_response()
        }
        Err(error) => {
            let error_response = JsonRpcResponse::error(request.id, to_jsonrpc_error(error));
            (StatusCode::OK, Json(error_response)).into_response()
        }
    }
}

fn handle_message_send(state: &AxumState, params: Option<Value>) -> crate::core::Result<Value> {
    let params = params.ok_or_else(|| A2aError::from_code(ErrorCode::InvalidParams, "missing params"))?;
    let payload: MessageSendParams = serde_json::from_value(params)?;
    let response = state.manager.send_message(payload)?;
    to_value(response)
}

async fn handle_message_stream(
    state: &AxumState,
    params: Option<Value>,
) -> crate::core::Result<Vec<String>> {
    let params = params.ok_or_else(|| A2aError::from_code(ErrorCode::InvalidParams, "missing params"))?;
    let payload: MessageSendParams = serde_json::from_value(params)?;
    let response = state.manager.send_message(payload)?;
    Ok(vec![
        serde_json::to_string(&serde_json::json!({"type": "start"}))?,
        serde_json::to_string(&response)?,
        serde_json::to_string(&serde_json::json!({"type": "done"}))?,
    ])
}

fn handle_task_get(state: &AxumState, params: Option<Value>) -> crate::core::Result<Value> {
    let params = params.ok_or_else(|| A2aError::from_code(ErrorCode::InvalidParams, "missing params"))?;
    let payload: TaskIdParams = serde_json::from_value(params)?;
    let task = state.manager.get_task(&payload.id)?;
    Ok(serde_json::to_value(task)?)
}

fn handle_task_cancel(state: &AxumState, params: Option<Value>) -> crate::core::Result<Value> {
    let params = params.ok_or_else(|| A2aError::from_code(ErrorCode::InvalidParams, "missing params"))?;
    let payload: TaskIdParams = serde_json::from_value(params)?;
    let task = state.manager.cancel_task(&payload.id)?;
    Ok(serde_json::to_value(task)?)
}

fn handle_task_resubscribe(state: &AxumState, params: Option<Value>) -> crate::core::Result<Vec<String>> {
    let params = params.ok_or_else(|| A2aError::from_code(ErrorCode::InvalidParams, "missing params"))?;
    let payload: TaskIdParams = serde_json::from_value(params)?;
    let task = state.manager.get_task(&payload.id)?;
    Ok(vec![
        serde_json::to_string(&serde_json::json!({"type": "start"}))?,
        serde_json::to_string(&task)?,
        serde_json::to_string(&serde_json::json!({"type": "done"}))?,
    ])
}

fn to_value(response: A2aResponse) -> crate::core::Result<Value> {
    Ok(serde_json::to_value(response)?)
}

fn to_jsonrpc_error(error: A2aError) -> JsonRpcError {
    match error {
        A2aError::Rpc { code, message, data } => JsonRpcError {
            code,
            message,
            data: data.map(Value::String),
        },
        A2aError::Json(message) => JsonRpcError {
            code: ErrorCode::InvalidParams.as_i32(),
            message,
            data: None,
        },
        A2aError::Protocol(message) => JsonRpcError {
            code: ErrorCode::InvalidRequest.as_i32(),
            message,
            data: None,
        },
        A2aError::Task(message) => JsonRpcError {
            code: ErrorCode::TaskNotFound.as_i32(),
            message,
            data: None,
        },
        A2aError::Http(message) => JsonRpcError {
            code: ErrorCode::InternalError.as_i32(),
            message,
            data: None,
        },
        A2aError::Timeout => JsonRpcError {
            code: ErrorCode::InternalError.as_i32(),
            message: "timeout".to_string(),
            data: None,
        },
    }
}
