use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use crossbeam_channel::Sender;
use serde_json::{json, Value};

use crate::test_system::channel::TestMessage;

use super::dispatch::call_tool;
use super::protocol::{RpcReq, RpcResp};
use super::tools::tool_list;

pub async fn mcp_handler(
    State(sender): State<Sender<TestMessage>>,
    Json(req): Json<RpcReq>,
) -> impl IntoResponse {
    let id = req.id.clone();

    match req.method.as_str() {
        "initialize" => Json(RpcResp::ok(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "bevy-game", "version": "1.0.0" }
            }),
        ))
        .into_response(),

        "notifications/initialized" => StatusCode::NO_CONTENT.into_response(),

        "tools/list" => Json(RpcResp::ok(id, tool_list())).into_response(),

        "tools/call" => {
            let params = req.params.as_ref().and_then(|v| v.as_object());
            let name = params
                .and_then(|m| m.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let args = params
                .and_then(|m| m.get("arguments"))
                .cloned()
                .unwrap_or(json!({}));

            let resp = match call_tool(&sender, name, &args).await {
                Ok(data) => RpcResp::ok(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&data).unwrap_or_default()
                        }]
                    }),
                ),
                Err(e) => RpcResp::err(id, -32603, e),
            };
            Json(resp).into_response()
        }

        unknown => Json(RpcResp::err(id, -32601, format!("未知方法: {}", unknown))).into_response(),
    }
}
