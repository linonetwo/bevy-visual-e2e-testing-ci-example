use crossbeam_channel::{Receiver, Sender};
use std::sync::OnceLock;
use tokio::sync::oneshot;

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
