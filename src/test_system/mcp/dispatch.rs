use crossbeam_channel::Sender;
use serde_json::{json, Value};
use tokio::sync::oneshot;

use crate::test_system::channel::TestMessage;

const TIMEOUT: u64 = 30;
const SCREENSHOT_TIMEOUT: u64 = 10;

async fn send<T: Send + 'static>(
    tx: &Sender<TestMessage>,
    make: impl FnOnce(oneshot::Sender<T>) -> TestMessage,
    timeout: u64,
) -> Result<T, String> {
    let (s, r) = oneshot::channel();
    tx.send(make(s)).map_err(|e| format!("发送失败: {}", e))?;
    tokio::time::timeout(tokio::time::Duration::from_secs(timeout), r)
        .await
        .map_err(|_| "超时".to_string())?
        .map_err(|_| "接收失败".to_string())
}

pub async fn call_tool(
    sender: &Sender<TestMessage>,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    macro_rules! str_arg {
        ($k:literal) => {
            args[$k]
                .as_str()
                .ok_or(concat!("缺少 ", $k))?
                .to_string()
        };
    }
    macro_rules! f32_arg {
        ($k:literal) => {
            args[$k].as_f64().ok_or(concat!("缺少 ", $k))? as f32
        };
    }
    macro_rules! bool_result {
        ($expr:expr, $label:literal) => {{
            let ok = $expr;
            json!({ "success": ok, "message": format!("{}{}", if ok { "" } else { "失败: " }, $label) })
        }};
    }

    Ok(match name {
        "health" => json!({ "status": "OK" }),
        "take_snapshot" => {
            let nodes = send(sender, |tx| TestMessage::TakeSnapshot { response: tx }, TIMEOUT).await?;
            nodes
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
                .into()
        }
        "component_counts" => {
            let mut list: Vec<Value> = send(
                sender,
                |tx| TestMessage::QueryComponents { response: tx },
                TIMEOUT,
            )
            .await?
            .into_iter()
            .map(|(n, c)| json!({ "name": n, "count": c }))
            .collect();
            list.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
            list.into()
        }
        "screenshot" => {
            let path = str_arg!("path");
            let ok = send(
                sender,
                |tx| TestMessage::Screenshot {
                    path: path.clone(),
                    response: tx,
                },
                SCREENSHOT_TIMEOUT,
            )
            .await?;
            json!({ "success": ok, "message": if ok { format!("截图完成: {}", path) } else { format!("截图失败: {}", path) } })
        }
        "click" => {
            let (x, y) = (f32_arg!("x"), f32_arg!("y"));
            bool_result!(
                send(sender, |tx| TestMessage::Click { x, y, response: tx }, TIMEOUT).await?,
                "click"
            )
        }
        "hover" => {
            let (x, y) = (f32_arg!("x"), f32_arg!("y"));
            bool_result!(
                send(sender, |tx| TestMessage::Hover { x, y, response: tx }, TIMEOUT).await?,
                "hover"
            )
        }
        "click_by_id" => {
            let id = str_arg!("id");
            bool_result!(
                send(sender, |tx| TestMessage::ClickById { id, response: tx }, TIMEOUT).await?,
                "click_by_id"
            )
        }
        "hover_by_id" => {
            let id = str_arg!("id");
            bool_result!(
                send(sender, |tx| TestMessage::HoverById { id, response: tx }, TIMEOUT).await?,
                "hover_by_id"
            )
        }
        "click_button" => {
            let button_name = str_arg!("button_name");
            bool_result!(
                send(
                    sender,
                    |tx| TestMessage::ClickButtonByName {
                        button_name,
                        response: tx,
                    },
                    TIMEOUT,
                )
                .await?,
                "click_button"
            )
        }
        "press_key" => {
            let key = str_arg!("key");
            bool_result!(
                send(sender, |tx| TestMessage::PressKey { key, response: tx }, TIMEOUT).await?,
                "press_key"
            )
        }
        "fill" => {
            let id = str_arg!("id");
            let value = str_arg!("value");
            bool_result!(
                send(
                    sender,
                    |tx| TestMessage::FillText {
                        id,
                        value,
                        response: tx,
                    },
                    TIMEOUT,
                )
                .await?,
                "fill"
            )
        }
        "drag" => {
            let from_id = str_arg!("from_id");
            let to_id = str_arg!("to_id");
            bool_result!(
                send(
                    sender,
                    |tx| TestMessage::Drag {
                        from_id,
                        to_id,
                        response: tx,
                    },
                    TIMEOUT,
                )
                .await?,
                "drag"
            )
        }
        "console_messages" => {
            let lines = args["lines"].as_u64().unwrap_or(50) as u32;
            let log_file = args["log_file"].as_str().map(String::from);
            send(
                sender,
                |tx| TestMessage::GetLogs {
                    lines,
                    log_file,
                    response: tx,
                },
                TIMEOUT,
            )
            .await?
            .into_iter()
            .map(|e| json!({ "timestamp": e.timestamp, "level": e.level, "message": e.message }))
            .collect::<Vec<_>>()
            .into()
        }
        "evaluate_script" => {
            let script = str_arg!("script");
            let result = send(
                sender,
                |tx| TestMessage::EvaluateScript {
                    script,
                    response: tx,
                },
                TIMEOUT,
            )
            .await?;
            json!({ "result": result })
        }
        unknown => return Err(format!("未知工具: {}", unknown)),
    })
}
