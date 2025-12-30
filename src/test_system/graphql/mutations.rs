use super::types::CommandResult;
use async_graphql::{Context, Object};
use crossbeam_channel::Sender;
use tokio::sync::oneshot;

use crate::test_system::channel::TestMessage;

const COMMAND_TIMEOUT_SECS: u64 = 2;
const SCREENSHOT_TIMEOUT_SECS: u64 = 5;

#[derive(Default)]
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn hover(&self, ctx: &Context<'_>, x: f32, y: f32) -> async_graphql::Result<CommandResult> {
        let sender = ctx.data::<Sender<TestMessage>>()?.clone();
        let (tx, rx) = oneshot::channel();

        sender
            .send(TestMessage::Hover { x, y, response: tx })
            .map_err(|e| async_graphql::Error::new(format!("发送消息失败: {}", e)))?;

        wait_bool(rx, COMMAND_TIMEOUT_SECS, "悬停").await
    }

    async fn click(&self, ctx: &Context<'_>, x: f32, y: f32) -> async_graphql::Result<CommandResult> {
        let sender = ctx.data::<Sender<TestMessage>>()?.clone();
        let (tx, rx) = oneshot::channel();

        sender
            .send(TestMessage::Click { x, y, response: tx })
            .map_err(|e| async_graphql::Error::new(format!("发送消息失败: {}", e)))?;

        wait_bool(rx, COMMAND_TIMEOUT_SECS, "点击").await
    }

    async fn screenshot(&self, ctx: &Context<'_>, path: String) -> async_graphql::Result<CommandResult> {
        let sender = ctx.data::<Sender<TestMessage>>()?.clone();
        let (tx, rx) = oneshot::channel();

        sender
            .send(TestMessage::Screenshot {
                path: path.clone(),
                response: tx,
            })
            .map_err(|e| async_graphql::Error::new(format!("发送消息失败: {}", e)))?;

        let result = tokio::time::timeout(tokio::time::Duration::from_secs(SCREENSHOT_TIMEOUT_SECS), rx)
            .await
            .map_err(|_| async_graphql::Error::new("截图超时"))?
            .map_err(|_| async_graphql::Error::new("接收确认失败"))?;

        Ok(CommandResult {
            success: result,
            message: if result {
                format!("截图完成: {}", path)
            } else {
                format!("截图失败: {}", path)
            },
        })
    }
}

async fn wait_bool(
    rx: oneshot::Receiver<bool>,
    timeout_secs: u64,
    action_name: &str,
) -> async_graphql::Result<CommandResult> {
    let result = tokio::time::timeout(tokio::time::Duration::from_secs(timeout_secs), rx)
        .await
        .map_err(|_| async_graphql::Error::new(format!("{}: 超时", action_name)))?
        .map_err(|_| async_graphql::Error::new(format!("{}: 接收确认失败", action_name)))?;

    Ok(CommandResult {
        success: result,
        message: if result {
            format!("{}完成", action_name)
        } else {
            format!("{}失败", action_name)
        },
    })
}
