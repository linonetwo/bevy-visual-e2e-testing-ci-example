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

// ---- 数据结构（供 channel 传输） ----

/// UI 快照节点数据（扁平结构，客户端可自行构建树）
#[derive(Clone, Debug, Default)]
pub struct UINodeData {
    /// 唯一标识符，格式 "bits:{entity_bits}"
    pub uid: String,
    /// Name 组件值，否则与 uid 相同
    pub name: String,
    /// "button" | "text" | "container" | "root"
    pub node_type: String,
    /// Text 组件内容
    pub text: Option<String>,
    /// TestId 组件值
    pub test_id: Option<String>,
    /// 是否可见
    pub visible: bool,
    /// 屏幕像素坐标 X
    pub x: f32,
    /// 屏幕像素坐标 Y
    pub y: f32,
    /// 计算后宽度（像素）
    pub width: f32,
    /// 计算后高度（像素）
    pub height: f32,
    /// 父节点 uid（根节点为 None）
    pub parent_uid: Option<String>,
}

/// 日志条目数据
#[derive(Clone, Debug, Default)]
pub struct LogEntryData {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

// ---- 测试消息类型 ----

#[derive(Debug)]
#[allow(dead_code)]
pub enum TestMessage {
    // ---- 原有消息 ----
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

    // ---- CDP 风格 UI 快照 ----

    /// 获取 UI 节点树快照（类似 CDP take_snapshot / a11y 树）
    TakeSnapshot {
        response: oneshot::Sender<Vec<UINodeData>>,
    },

    // ---- 按 ID 操作元素 ----

    /// 按 test_id / Name / "bits:{n}" 点击元素
    ClickById {
        id: String,
        response: oneshot::Sender<bool>,
    },
    /// 按 test_id / Name / "bits:{n}" 悬停元素
    HoverById {
        id: String,
        response: oneshot::Sender<bool>,
    },
    /// 按 test_id / Name / "bits:{n}" 点击按钮（名称匹配）
    ClickButtonByName {
        button_name: String,
        response: oneshot::Sender<bool>,
    },

    // ---- 键盘 / 文本输入 ----

    /// 模拟按键（支持 "Space" / "Enter" / "Escape" / "ArrowUp" 等）
    PressKey {
        key: String,
        response: oneshot::Sender<bool>,
    },
    /// 向元素填充文本（先清空再写入）
    FillText {
        id: String,
        value: String,
        response: oneshot::Sender<bool>,
    },

    // ---- 拖拽 ----

    /// 模拟拖拽：从 from_id 拖到 to_id
    Drag {
        from_id: String,
        to_id: String,
        response: oneshot::Sender<bool>,
    },

    // ---- 日志 / 脚本 ----

    /// 读取后端日志文件，返回最近 N 行
    GetLogs {
        lines: u32,
        log_file: Option<String>,
        response: oneshot::Sender<Vec<LogEntryData>>,
    },
    /// 在 Tauri 前端执行 JavaScript（需设置 JS_EVALUATOR_URL 环境变量）
    EvaluateScript {
        script: String,
        response: oneshot::Sender<String>,
    },
}
