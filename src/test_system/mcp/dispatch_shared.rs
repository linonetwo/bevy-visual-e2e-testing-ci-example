//! 各 dispatch 子模块共享的常量与辅助函数

use crossbeam_channel::Sender;
use serde_json::{json, Value};
use tokio::sync::oneshot;

use crate::test_system::channel::TestMessage;

pub const TIMEOUT: u64 = 30;
pub const SCREENSHOT_TIMEOUT: u64 = 10;

/// 向 Bevy 主线程发送消息并等待响应
pub async fn send<T: Send + 'static>(
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

/// 从 JSON args 中取字符串参数
pub fn arg_str(args: &Value, k: &str) -> Result<String, String> {
    args[k]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("缺少参数: {}", k))
}

/// 从 JSON args 中取 f32 数值参数
pub fn arg_f32(args: &Value, k: &str) -> Result<f32, String> {
    args[k]
        .as_f64()
        .map(|v| v as f32)
        .ok_or_else(|| format!("缺少参数: {}", k))
}

/// 构造布尔型操作结果
pub fn bool_cmd(label: &str, ok: bool) -> Value {
    json!({
        "success": ok,
        "message": format!("{}{}", if ok { "" } else { "失败: " }, label)
    })
}
