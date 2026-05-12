use a2a_rust_sdk::core::{JsonRpcRequest, JsonRpcResponse};
use serde_json::json;

#[test]
fn jsonrpc_request_round_trip() {
    let request = JsonRpcRequest::new(json!(1), "message/send", Some(json!({"key": "value"})));
    let text = serde_json::to_string(&request).expect("serialize");
    let decoded: JsonRpcRequest = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(decoded.method, "message/send");
}

#[test]
fn jsonrpc_response_round_trip() {
    let response = JsonRpcResponse::success(json!("req-1"), json!({"ok": true}));
    let text = serde_json::to_string(&response).expect("serialize");
    let decoded: JsonRpcResponse = serde_json::from_str(&text).expect("deserialize");
    assert!(decoded.result.is_some());
}
