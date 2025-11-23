use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::sync::oneshot;

// 超时配置常量
const COMMAND_TIMEOUT_SECS: u64 = 2;
const SCREENSHOT_TIMEOUT_SECS: u64 = 5;

// 全局消息通道，供游戏主循环接收命令
pub static TEST_COMMAND_CHANNEL: OnceLock<TestChannel> = OnceLock::new();

// 测试命令通道
#[allow(dead_code)]
pub struct TestChannel {
    pub sender: Sender<TestMessage>,
    pub receiver: Receiver<TestMessage>,
}

// 测试消息类型
#[derive(Debug)]
#[allow(dead_code)]
pub enum TestMessage {
    Hover {
        x: f32,
        y: f32,
        response: oneshot::Sender<bool>,
    },
    Click {
        x: f32,
        y: f32,
        response: oneshot::Sender<bool>,
    },
    Screenshot {
        path: String,
        response: oneshot::Sender<bool>,
    },
    QueryComponents {
        response: oneshot::Sender<std::collections::HashMap<String, usize>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCommand {
    pub action: String,
    pub selector: Option<String>,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

pub fn start_test_server() {
    std::thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            // 创建无界通道
            let (sender, receiver) = unbounded();

            // 设置全局通道
            let _ = TEST_COMMAND_CHANNEL.set(TestChannel {
                sender: sender.clone(),
                receiver,
            });

            let app = Router::new()
                .route("/health", get(health_check))
                .route("/ws", get(ws_handler))
                .with_state(sender);

            // 从环境变量获取端口，默认为 9222
            let port = std::env::var("TEST_PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(9222);

            let addr = format!("127.0.0.1:{}", port);
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

            info!("测试自动化服务器启动在 http://{}", addr);

            axum::serve(listener, app).await.unwrap();
        });
    });
}

async fn health_check() -> impl IntoResponse {
    "OK"
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(sender): State<Sender<TestMessage>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, sender))
}

async fn handle_socket(mut socket: WebSocket, sender: Sender<TestMessage>) {
    info!("WebSocket 连接已建立");

    while let Some(msg) = socket.recv().await {
        if let Ok(axum::extract::ws::Message::Text(text)) = msg {
            info!("收到消息: {}", text);

            if let Ok(cmd) = serde_json::from_str::<TestCommand>(&text) {
                let response = handle_command(cmd, sender.clone()).await;
                let response_text = serde_json::to_string(&response).unwrap();
                let _ = socket
                    .send(axum::extract::ws::Message::Text(response_text.into()))
                    .await;
            }
        }
    }

    info!("WebSocket 连接已关闭");
}

async fn handle_command(cmd: TestCommand, sender: Sender<TestMessage>) -> TestResponse {
    info!("处理命令: {:?}", cmd);

    match cmd.action.as_str() {
        "hover" => {
            handle_coordinate_command(
                cmd.params,
                |x, y, tx| TestMessage::Hover { x, y, response: tx },
                sender,
                "悬停",
            )
            .await
        }
        "click" => {
            handle_coordinate_command(
                cmd.params,
                |x, y, tx| TestMessage::Click { x, y, response: tx },
                sender,
                "点击",
            )
            .await
        }
        "screenshot" => handle_screenshot_command(cmd.params, sender).await,
        "query_components" => handle_query_components(sender).await,
        _ => TestResponse {
            success: false,
            message: format!("未知命令: {}", cmd.action),
            data: None,
        },
    }
}

// 处理需要坐标的命令（hover, click）
async fn handle_coordinate_command<F>(
    params: Option<serde_json::Value>,
    create_message: F,
    sender: Sender<TestMessage>,
    action_name: &str,
) -> TestResponse
where
    F: FnOnce(f32, f32, oneshot::Sender<bool>) -> TestMessage,
{
    let Some(params) = params else {
        return TestResponse {
            success: false,
            message: "缺少参数".to_string(),
            data: None,
        };
    };

    let (Some(x), Some(y)) = (
        params.get("x").and_then(|v| v.as_f64()),
        params.get("y").and_then(|v| v.as_f64()),
    ) else {
        return TestResponse {
            success: false,
            message: "缺少 x, y 坐标".to_string(),
            data: None,
        };
    };

    let (tx, rx) = oneshot::channel();

    if let Err(e) = sender.send(create_message(x as f32, y as f32, tx)) {
        return TestResponse {
            success: false,
            message: format!("发送消息失败: {}", e),
            data: None,
        };
    }

    info!("发送{}消息: ({}, {})", action_name, x, y);

    wait_for_response(rx, COMMAND_TIMEOUT_SECS, action_name).await
}

// 处理截图命令
async fn handle_screenshot_command(
    params: Option<serde_json::Value>,
    sender: Sender<TestMessage>,
) -> TestResponse {
    let Some(params) = params else {
        return error_response("缺少参数");
    };

    let Some(path) = params.get("path").and_then(|v| v.as_str()) else {
        return error_response("缺少 path 参数");
    };

    let (tx, rx) = oneshot::channel();

    if let Err(e) = sender.send(TestMessage::Screenshot {
        path: path.to_string(),
        response: tx,
    }) {
        return error_response(&format!("发送消息失败: {}", e));
    }

    info!("发送截图请求: {}", path);

    match wait_for_bool_response(rx, SCREENSHOT_TIMEOUT_SECS).await {
        Ok(true) => TestResponse {
            success: true,
            message: format!("截图完成: {}", path),
            data: None,
        },
        Ok(false) => error_response(&format!("截图失败: {}", path)),
        Err(msg) => error_response(&msg),
    }
}

// 处理组件查询
async fn handle_query_components(sender: Sender<TestMessage>) -> TestResponse {
    let (tx, rx) = oneshot::channel();

    if let Err(e) = sender.send(TestMessage::QueryComponents { response: tx }) {
        return error_response(&format!("发送查询失败: {}", e));
    }

    match tokio::time::timeout(tokio::time::Duration::from_secs(COMMAND_TIMEOUT_SECS), rx).await {
        Ok(Ok(counts)) => TestResponse {
            success: true,
            message: "组件查询完成".to_string(),
            data: Some(serde_json::json!(counts)),
        },
        Ok(Err(_)) => error_response("接收响应失败"),
        Err(_) => error_response("查询超时"),
    }
}

// 辅助函数：等待 bool 响应
async fn wait_for_bool_response(
    rx: oneshot::Receiver<bool>,
    timeout_secs: u64,
) -> Result<bool, String> {
    match tokio::time::timeout(tokio::time::Duration::from_secs(timeout_secs), rx).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(_)) => Err("接收确认失败".to_string()),
        Err(_) => Err("超时".to_string()),
    }
}

// 辅助函数：等待并格式化响应
async fn wait_for_response(
    rx: oneshot::Receiver<bool>,
    timeout_secs: u64,
    action_name: &str,
) -> TestResponse {
    match wait_for_bool_response(rx, timeout_secs).await {
        Ok(true) => TestResponse {
            success: true,
            message: format!("{}完成", action_name),
            data: None,
        },
        Ok(false) => error_response(&format!("{}失败", action_name)),
        Err(msg) => error_response(&format!("{}: {}", action_name, msg)),
    }
}

// 辅助函数：创建错误响应
fn error_response(message: &str) -> TestResponse {
    TestResponse {
        success: false,
        message: message.to_string(),
        data: None,
    }
}
