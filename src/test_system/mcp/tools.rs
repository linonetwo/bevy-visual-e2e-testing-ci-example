use serde_json::{json, Value};

pub fn tool_list() -> Value {
    json!({
        "tools": [
            {
                "name": "health",
                "description": "检查游戏测试服务器是否运行",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "take_snapshot",
                "description": "获取游戏 UI 节点树快照（类 CDP take_snapshot）。返回所有节点：uid、name、nodeType、text、testId、visible、x/y/width/height、parentUid。",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "component_counts",
                "description": "查询游戏中各组件的实体数量（Ball、Button 等）",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "screenshot",
                "description": "截取游戏画面并保存到指定路径",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "保存路径，如 screenshots/test.png" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "click",
                "description": "在屏幕坐标 (x, y) 处点击",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "x": { "type": "number", "description": "屏幕 X 坐标（像素）" },
                        "y": { "type": "number", "description": "屏幕 Y 坐标（像素）" }
                    },
                    "required": ["x", "y"]
                }
            },
            {
                "name": "hover",
                "description": "将鼠标悬停在屏幕坐标 (x, y) 处",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "x": { "type": "number" },
                        "y": { "type": "number" }
                    },
                    "required": ["x", "y"]
                }
            },
            {
                "name": "click_by_id",
                "description": "按 test_id / Name / uid(bits:xxxx) 点击 UI 元素（类 CDP click(uid)）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "元素标识：testId / Name / bits:xxxx" }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "hover_by_id",
                "description": "按 test_id / Name / uid(bits:xxxx) 悬停 UI 元素",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "click_button",
                "description": "按按钮的 Name 或 test_id 点击按钮",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "button_name": { "type": "string", "description": "按钮名称，如 main-button" }
                    },
                    "required": ["button_name"]
                }
            },
            {
                "name": "press_key",
                "description": "模拟键盘按键。支持：Space、Enter、Escape、Tab、Backspace、Delete、ArrowUp/Down/Left/Right、F1-F12、KeyA-Z、Digit0-9",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "key": { "type": "string", "description": "按键名，如 Space、Enter、ArrowUp" }
                    },
                    "required": ["key"]
                }
            },
            {
                "name": "fill",
                "description": "向指定 UI 元素（Text 组件）填充文本（先清空原内容）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "元素标识" },
                        "value": { "type": "string", "description": "要填充的文本" }
                    },
                    "required": ["id", "value"]
                }
            },
            {
                "name": "drag",
                "description": "从源元素拖拽到目标元素",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "from_id": { "type": "string" },
                        "to_id": { "type": "string" }
                    },
                    "required": ["from_id", "to_id"]
                }
            },
            {
                "name": "console_messages",
                "description": "读取后端游戏日志文件，返回最近 N 行（类 CDP list_console_messages）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "lines": { "type": "integer", "description": "返回行数，默认 50", "default": 50 },
                        "log_file": { "type": "string", "description": "日志文件路径（可选）" }
                    }
                }
            },
            {
                "name": "evaluate_script",
                "description": "在 Tauri 前端 WebView 执行 JavaScript（需设置 JS_EVALUATOR_URL 环境变量）",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "script": { "type": "string", "description": "JavaScript 代码" }
                    },
                    "required": ["script"]
                }
            }
        ]
    })
}
