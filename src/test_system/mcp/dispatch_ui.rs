use crossbeam_channel::Sender;
use serde_json::{json, Value};

use crate::test_system::channel::TestMessage;

use super::dispatch_shared::{arg_f32, arg_str, bool_cmd, send, SCREENSHOT_TIMEOUT, TIMEOUT};

macro_rules! try_ok {
    ($expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        }
    };
}

pub async fn handle(
    sender: &Sender<TestMessage>,
    name: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    Some(match name {
        "health" => Ok(json!({ "status": "OK" })),

        "take_snapshot" => {
            let nodes = match send(
                sender,
                |tx| TestMessage::TakeSnapshot { response: tx },
                TIMEOUT,
            )
            .await
            {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };
            Ok(nodes
                .into_iter()
                .map(|n| {
                    json!({
                        "uid": n.uid,
                        "name": n.name,
                        "nodeType": n.node_type,
                        "text": n.text,
                        "testId": n.test_id,
                        "visible": n.visible,
                        "x": n.x,
                        "y": n.y,
                        "width": n.width,
                        "height": n.height,
                        "parentUid": n.parent_uid,
                    })
                })
                .collect::<Vec<_>>()
                .into())
        }

        "screenshot" => {
            let path = match arg_str(args, "path") {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };
            let ok = try_ok!(
                send(
                    sender,
                    |tx| TestMessage::Screenshot {
                        path: path.clone(),
                        response: tx
                    },
                    SCREENSHOT_TIMEOUT,
                )
                .await
            );
            if !ok {
                return Some(Ok(json!({
                    "success": false,
                    "path": path,
                    "message": format!("截图失败: {}", path)
                })));
            }
            // 对标 Chrome DevTools MCP screenshot.ts：
            // 读取文件字节并 base64 内联（永远内联，不做大小限制）
            match std::fs::read(&path) {
                Ok(bytes) => {
                    use base64::Engine;
                    let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    Ok(json!({
                        "__mcp_image": {
                            "data": data,
                            "mimeType": "image/png"
                        },
                        "text": format!("截图已保存: {}", path)
                    }))
                }
                Err(e) => Ok(json!({
                    "success": true,
                    "path": path,
                    "message": format!("截图已保存但无法读取文件: {}", e)
                })),
            }
        }

        "click" => {
            let x = try_ok!(arg_f32(args, "x"));
            let y = try_ok!(arg_f32(args, "y"));
            let ok = try_ok!(
                send(
                    sender,
                    |tx| TestMessage::Click { x, y, response: tx },
                    TIMEOUT
                )
                .await
            );
            Ok(bool_cmd("click", ok))
        }

        "hover" => {
            let x = try_ok!(arg_f32(args, "x"));
            let y = try_ok!(arg_f32(args, "y"));
            let ok = try_ok!(
                send(
                    sender,
                    |tx| TestMessage::Hover { x, y, response: tx },
                    TIMEOUT
                )
                .await
            );
            Ok(bool_cmd("hover", ok))
        }

        "click_by_id" => {
            let id = try_ok!(arg_str(args, "id"));
            let ok = try_ok!(
                send(
                    sender,
                    |tx| TestMessage::ClickById { id, response: tx },
                    TIMEOUT
                )
                .await
            );
            Ok(bool_cmd("click_by_id", ok))
        }

        "hover_by_id" => {
            let id = try_ok!(arg_str(args, "id"));
            let ok = try_ok!(
                send(
                    sender,
                    |tx| TestMessage::HoverById { id, response: tx },
                    TIMEOUT
                )
                .await
            );
            Ok(bool_cmd("hover_by_id", ok))
        }

        "click_button" => {
            let button_name = try_ok!(arg_str(args, "button_name"));
            let ok = try_ok!(
                send(
                    sender,
                    |tx| TestMessage::ClickButtonByName {
                        button_name,
                        response: tx
                    },
                    TIMEOUT
                )
                .await
            );
            Ok(bool_cmd("click_button", ok))
        }

        "press_key" => {
            let key = try_ok!(arg_str(args, "key"));
            let ok = try_ok!(
                send(
                    sender,
                    |tx| TestMessage::PressKey { key, response: tx },
                    TIMEOUT
                )
                .await
            );
            Ok(bool_cmd("press_key", ok))
        }

        "fill" => {
            let id = try_ok!(arg_str(args, "id"));
            let value = try_ok!(arg_str(args, "value"));
            let ok = try_ok!(
                send(
                    sender,
                    |tx| TestMessage::FillText {
                        id,
                        value,
                        response: tx
                    },
                    TIMEOUT
                )
                .await
            );
            Ok(bool_cmd("fill", ok))
        }

        "drag" => {
            let from_id = try_ok!(arg_str(args, "from_id"));
            let to_id = try_ok!(arg_str(args, "to_id"));
            let ok = try_ok!(
                send(
                    sender,
                    |tx| TestMessage::Drag {
                        from_id,
                        to_id,
                        response: tx
                    },
                    TIMEOUT
                )
                .await
            );
            Ok(bool_cmd("drag", ok))
        }

        _ => return None,
    })
}
