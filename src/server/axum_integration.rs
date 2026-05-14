use std::convert::Infallible;
use std::sync::Arc;
use std::time::{Duration, Instant};
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::HeaderValue;
use axum::http::Request;
use axum::http::StatusCode;
use axum::middleware;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::response::sse::{Event, Sse};
use axum::routing::get;
use axum::routing::post;
use serde_json::Value;
use tower_http::set_header::SetResponseHeaderLayer;
use tokio_stream::iter;
use tokio::sync::Mutex;

use crate::core::{A2aError, ErrorCode, JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::models::{A2aResponse, MessageSendParams, TaskIdParams};
use crate::server::TaskManager;

#[derive(Clone)]
pub struct AxumState {
    pub manager: Arc<TaskManager>,
}

pub fn axum_router(manager: Arc<TaskManager>) -> Router {
    let state = AxumState { manager };
    let cache_seconds = std::env::var("A2A_DISCOVERY_CACHE_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60);
    let cache_value = format!("public, max-age={}", cache_seconds);
    let cache_header = HeaderValue::from_str(&cache_value)
        .unwrap_or_else(|_| HeaderValue::from_static("public, max-age=60"));
    let cache_layer = SetResponseHeaderLayer::if_not_present(
        axum::http::header::CACHE_CONTROL,
        cache_header,
    );

    let rate_limit = std::env::var("A2A_DISCOVERY_RPS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5);
    let limiter = Arc::new(SimpleRateLimiter::new(rate_limit, Duration::from_secs(1)));
    let rate_layer = middleware::from_fn(move |req, next: Next| {
        let limiter = limiter.clone();
        async move {
            if !limiter.allow().await {
                return Ok::<Response, Infallible>(StatusCode::TOO_MANY_REQUESTS.into_response());
            }
            Ok(next.run(req).await)
        }
    });

    let discovery = Router::new().route(
        "/.well-known/agent-card.json",
        get(handle_agent_card).layer(cache_layer).layer(rate_layer),
    );

    Router::new()
        .route("/", post(handle_rpc))
        .route("/stream", post(handle_stream))
        .merge(discovery)
        .with_state(state)
}

async fn handle_agent_card(
    State(state): State<AxumState>,
    req: Request<axum::body::Body>,
) -> impl IntoResponse {
    let base_url = infer_base_url(&req);
    let card = state.manager.get_agent_card(&base_url);
    let authorized = is_authorized(&req);
    let response_card = if authorized { card } else { card.redacted() };
    (StatusCode::OK, axum::Json(response_card))
}

fn infer_base_url(req: &Request<axum::body::Body>) -> String {
    let headers = req.headers();

    let scheme = header_value(headers, "x-forwarded-proto")
        .or_else(|| req.uri().scheme_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "http".to_string());

    let mut host = header_value(headers, "x-forwarded-host")
        .or_else(|| header_value(headers, "host"))
        .unwrap_or_else(|| "127.0.0.1:5000".to_string());

    if !host.contains(':') {
        if let Some(port) = header_value(headers, "x-forwarded-port") {
            host = format!("{}:{}", host, port);
        }
    }

    format!("{}://{}", scheme, host)
}

fn is_authorized(req: &Request<axum::body::Body>) -> bool {
    let token = match std::env::var("A2A_AGENT_CARD_TOKEN") {
        Ok(value) if !value.is_empty() => value,
        _ => return false,
    };

    let header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let bearer = header.strip_prefix("Bearer ").unwrap_or("");
    bearer == token
}

struct SimpleRateLimiter {
    max: u64,
    window: Duration,
    state: Mutex<RateLimitState>,
}

struct RateLimitState {
    window_start: Instant,
    count: u64,
}

impl SimpleRateLimiter {
    fn new(max: u64, window: Duration) -> Self {
        Self {
            max,
            window,
            state: Mutex::new(RateLimitState {
                window_start: Instant::now(),
                count: 0,
            }),
        }
    }

    async fn allow(&self) -> bool {
        if self.max == 0 {
            return true;
        }

        let mut state = self.state.lock().await;
        if state.window_start.elapsed() >= self.window {
            state.window_start = Instant::now();
            state.count = 0;
        }

        if state.count >= self.max {
            return false;
        }

        state.count += 1;
        true
    }
}

fn header_value(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(',').next().unwrap_or(value).trim().to_string())
}

async fn handle_rpc(
    State(state): State<AxumState>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let result = match request.method.as_str() {
        "message/send" => handle_message_send(&state, request.params),
        "tasks/get" => handle_task_get(&state, request.params),
        "tasks/cancel" => handle_task_cancel(&state, request.params),
        _ => Err(A2aError::from_code(
            ErrorCode::MethodNotFound,
            "method not found",
        )),
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
        _ => Err(A2aError::from_code(
            ErrorCode::MethodNotFound,
            "method not found",
        )),
    };

    match result {
        Ok(payloads) => {
            let events = iter(
                payloads
                    .into_iter()
                    .map(|payload| Ok::<Event, Infallible>(Event::default().data(payload))),
            );
            Sse::new(events).into_response()
        }
        Err(error) => {
            let error_response = JsonRpcResponse::error(request.id, to_jsonrpc_error(error));
            (StatusCode::OK, Json(error_response)).into_response()
        }
    }
}

fn handle_message_send(state: &AxumState, params: Option<Value>) -> crate::core::Result<Value> {
    let params =
        params.ok_or_else(|| A2aError::from_code(ErrorCode::InvalidParams, "missing params"))?;
    let payload: MessageSendParams = serde_json::from_value(params)?;
    let response = state.manager.send_message(payload)?;
    to_value(response)
}

async fn handle_message_stream(
    state: &AxumState,
    params: Option<Value>,
) -> crate::core::Result<Vec<String>> {
    let params =
        params.ok_or_else(|| A2aError::from_code(ErrorCode::InvalidParams, "missing params"))?;
    let payload: MessageSendParams = serde_json::from_value(params)?;
    let response = state.manager.send_message(payload)?;
    Ok(vec![
        serde_json::to_string(&serde_json::json!({"type": "start"}))?,
        serde_json::to_string(&response)?,
        serde_json::to_string(&serde_json::json!({"type": "done"}))?,
    ])
}

fn handle_task_get(state: &AxumState, params: Option<Value>) -> crate::core::Result<Value> {
    let params =
        params.ok_or_else(|| A2aError::from_code(ErrorCode::InvalidParams, "missing params"))?;
    let payload: TaskIdParams = serde_json::from_value(params)?;
    let task = state.manager.get_task(&payload.id)?;
    Ok(serde_json::to_value(task)?)
}

fn handle_task_cancel(state: &AxumState, params: Option<Value>) -> crate::core::Result<Value> {
    let params =
        params.ok_or_else(|| A2aError::from_code(ErrorCode::InvalidParams, "missing params"))?;
    let payload: TaskIdParams = serde_json::from_value(params)?;
    let task = state.manager.cancel_task(&payload.id)?;
    Ok(serde_json::to_value(task)?)
}

fn handle_task_resubscribe(
    state: &AxumState,
    params: Option<Value>,
) -> crate::core::Result<Vec<String>> {
    let params =
        params.ok_or_else(|| A2aError::from_code(ErrorCode::InvalidParams, "missing params"))?;
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
        A2aError::Rpc {
            code,
            message,
            data,
        } => JsonRpcError {
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
