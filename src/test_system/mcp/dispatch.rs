//! MCP 工具调度编排层
//!
//! 具体实现分散在：
//! - dispatch_ui.rs      UI 交互：take_snapshot、screenshot、click、hover、fill、drag 等
//! - dispatch_system.rs  系统/调试：component_counts、console_messages、evaluate_script
//! - dispatch_shared.rs  共享常量与辅助函数

use crossbeam_channel::Sender;
use serde_json::Value;

use crate::test_system::channel::TestMessage;

#[path = "dispatch_shared.rs"]
mod dispatch_shared;
#[path = "dispatch_system.rs"]
mod dispatch_system;
#[path = "dispatch_ui.rs"]
mod dispatch_ui;

pub async fn call_tool(
    sender: &Sender<TestMessage>,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    if let Some(result) = dispatch_ui::handle(sender, name, args).await {
        return result;
    }
    if let Some(result) = dispatch_system::handle(sender, name, args).await {
        return result;
    }
    Err(format!("未知工具: {}", name))
}
