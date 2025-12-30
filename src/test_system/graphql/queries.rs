use super::types::ComponentCount;
use async_graphql::{Context, Object};
use crossbeam_channel::Sender;
use tokio::sync::oneshot;

use crate::test_system::channel::TestMessage;

const COMMAND_TIMEOUT_SECS: u64 = 2;

#[derive(Default)]
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn health(&self) -> &str {
        "OK"
    }

    async fn component_counts(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<ComponentCount>> {
        let sender = ctx.data::<Sender<TestMessage>>()?.clone();

        let (tx, rx) = oneshot::channel();
        sender
            .send(TestMessage::QueryComponents { response: tx })
            .map_err(|e| async_graphql::Error::new(format!("发送查询失败: {}", e)))?;

        let counts = tokio::time::timeout(tokio::time::Duration::from_secs(COMMAND_TIMEOUT_SECS), rx)
            .await
            .map_err(|_| async_graphql::Error::new("查询超时"))?
            .map_err(|_| async_graphql::Error::new("接收响应失败"))?;

        let mut results: Vec<ComponentCount> = counts
            .into_iter()
            .map(|(name, count)| ComponentCount {
                name,
                count: count as i32,
            })
            .collect();

        results.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(results)
    }
}
