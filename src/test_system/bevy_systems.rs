use bevy::prelude::*;
use log::info;

use crate::{Ball, GameButton, TestId};
use crate::test_system::channel::{LogEntryData, TestMessage, UINodeData, TEST_COMMAND_CHANNEL};

// 从消息队列接收测试消息并直接处理
pub fn receive_test_messages(
    mut commands: Commands,
    mut button_query: Query<&mut Interaction, With<GameButton>>,
    ball_query: Query<&Ball>,
    button_count_query: Query<&GameButton>,
    mut keyboard_input: ResMut<ButtonInput<KeyCode>>,
) {
    if let Some(channel) = TEST_COMMAND_CHANNEL.get() {
        // 非阻塞地接收所有待处理消息
        while let Ok(msg) = channel.receiver.try_recv() {
            match msg {
                // ---- 原有消息 ----

                TestMessage::Hover { x, y, response } => {
                    info!("收到测试悬停消息: ({}, {})", x, y);
                    for mut interaction in button_query.iter_mut() {
                        *interaction = Interaction::Hovered;
                    }
                    let _ = response.send(true);
                }
                TestMessage::Click { x, y, response } => {
                    info!("收到测试点击消息: ({}, {})", x, y);
                    for mut interaction in button_query.iter_mut() {
                        *interaction = Interaction::Pressed;
                    }
                    let _ = response.send(true);
                }
                TestMessage::Screenshot { path, response } => {
                    info!("收到截图请求: {}", path);
                    let path_clone = path.clone();

                    commands
                        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                        .observe(bevy::render::view::screenshot::save_to_disk(path));

                    std::thread::spawn(move || {
                        use backoff::ExponentialBackoffBuilder;
                        use std::time::Duration;

                        let backoff_config = ExponentialBackoffBuilder::new()
                            .with_initial_interval(Duration::from_millis(50))
                            .with_max_interval(Duration::from_millis(500))
                            .with_max_elapsed_time(Some(Duration::from_secs(5)))
                            .build();

                        let result = backoff::retry(backoff_config, || {
                            let path = std::path::Path::new(&path_clone);
                            if path.exists() {
                                // 等文件写入完成（size > 0），避免 base64 读到空数据
                                match std::fs::metadata(&path_clone) {
                                    Ok(m) if m.len() > 0 => Ok(()),
                                    _ => Err(backoff::Error::transient("文件未写完")),
                                }
                            } else {
                                Err(backoff::Error::transient("文件未生成"))
                            }
                        });

                        let _ = response.send(result.is_ok());
                    });
                }
                TestMessage::QueryComponents { response } => {
                    info!("收到组件查询消息");
                    let ball_count = ball_query.iter().count();
                    let button_count = button_count_query.iter().count();

                    let mut counts = std::collections::HashMap::new();
                    counts.insert("Ball".to_string(), ball_count);
                    counts.insert("Button".to_string(), button_count);

                    info!(
                        "COMPONENT_COUNTS: Ball={}, Button={}",
                        ball_count, button_count
                    );
                    let _ = response.send(counts);
                }

                // ---- CDP 风格：UI 快照 ----

                TestMessage::TakeSnapshot { response } => {
                    info!("收到 UI 快照请求");
                    commands.queue(move |world: &mut World| {
                        let nodes = build_ui_snapshot(world);
                        info!("UI 快照节点数: {}", nodes.len());
                        let _ = response.send(nodes);
                    });
                }

                // ---- 按 ID 操作元素 ----

                TestMessage::ClickById { id, response } => {
                    info!("收到 ClickById: {}", id);
                    commands.queue(move |world: &mut World| {
                        let found = find_entity_and_set_interaction(
                            world,
                            &id,
                            Interaction::Pressed,
                        );
                        let _ = response.send(found);
                    });
                }
                TestMessage::HoverById { id, response } => {
                    info!("收到 HoverById: {}", id);
                    commands.queue(move |world: &mut World| {
                        let found = find_entity_and_set_interaction(
                            world,
                            &id,
                            Interaction::Hovered,
                        );
                        let _ = response.send(found);
                    });
                }
                TestMessage::ClickButtonByName { button_name, response } => {
                    info!("收到 ClickButtonByName: {}", button_name);
                    commands.queue(move |world: &mut World| {
                        let found = find_entity_and_set_interaction(
                            world,
                            &button_name,
                            Interaction::Pressed,
                        );
                        let _ = response.send(found);
                    });
                }

                // ---- 键盘 / 文本输入 ----

                TestMessage::PressKey { key, response } => {
                    info!("收到 PressKey: {}", key);
                    let success = if let Some(key_code) = parse_key_code(&key) {
                        keyboard_input.press(key_code);
                        info!("PressKey 成功: {:?}", key_code);
                        true
                    } else {
                        info!("未知按键名称: {}", key);
                        false
                    };
                    let _ = response.send(success);
                }
                TestMessage::FillText { id, value, response } => {
                    info!("收到 FillText: {} = '{}'", id, value);
                    let value_clone = value.clone();
                    commands.queue(move |world: &mut World| {
                        if let Some(entity) = find_entity_by_test_id(world, &id) {
                            if let Some(mut text) = world.get_mut::<Text>(entity) {
                                *text = Text::new(&value_clone);
                                info!("FillText 成功: entity={:?}", entity);
                                let _ = response.send(true);
                                return;
                            }
                        }
                        info!("FillText 失败: 未找到 id={}", id);
                        let _ = response.send(false);
                    });
                }

                // ---- 拖拽 ----

                TestMessage::Drag { from_id, to_id, response } => {
                    info!("收到 Drag: {} -> {}", from_id, to_id);
                    commands.queue(move |world: &mut World| {
                        // 模拟：按下源元素，悬停目标元素
                        let success = find_entity_and_set_interaction(
                            world,
                            &from_id,
                            Interaction::Pressed,
                        );
                        find_entity_and_set_interaction(
                            world,
                            &to_id,
                            Interaction::Hovered,
                        );
                        let _ = response.send(success);
                    });
                }

                // ---- 日志 / 脚本 ----

                TestMessage::GetLogs { lines, log_file, response } => {
                    info!("收到 GetLogs: lines={}", lines);
                    std::thread::spawn(move || {
                        let file_path = log_file.unwrap_or_else(|| {
                            std::env::var("TEST_LOG_FILE")
                                .unwrap_or_else(|_| "logs/game.log".to_string())
                        });
                        let entries = read_log_file(&file_path, lines);
                        let _ = response.send(entries);
                    });
                }
                TestMessage::EvaluateScript { script, response } => {
                    info!("收到 EvaluateScript");
                    std::thread::spawn(move || {
                        // 若设置了 JS_EVALUATOR_URL，向 Tauri IPC 端点 POST 脚本
                        if let Ok(url) = std::env::var("JS_EVALUATOR_URL") {
                            // 简单 HTTP POST（需要游戏侧有对应端点）
                            let result = std::process::Command::new("curl")
                                .args([
                                    "-s",
                                    "-X", "POST",
                                    "-H", "Content-Type: application/json",
                                    "-d", &format!("{{\"script\":{}}}", serde_json::to_string(&script).unwrap_or_default()),
                                    &url,
                                ])
                                .output()
                                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                                .unwrap_or_else(|e| format!("{{\"error\":\"curl 失败: {}\"}}", e));
                            let _ = response.send(result);
                        } else {
                            let _ = response.send(
                                "{\"error\":\"未配置 JS_EVALUATOR_URL 环境变量，Tauri 集成需要设置此变量\"}".to_string()
                            );
                        }
                    });
                }
            }
        }
    }
}

// ============================================================
// 辅助函数
// ============================================================

/// 读取日志文件，返回最近 N 行解析后的条目
fn read_log_file(file_path: &str, lines: u32) -> Vec<LogEntryData> {
    match std::fs::read_to_string(file_path) {
        Ok(content) => {
            let all_lines: Vec<&str> = content.lines().collect();
            let start = all_lines.len().saturating_sub(lines as usize);
            all_lines[start..]
                .iter()
                .map(|line| parse_log_line(line))
                .collect()
        }
        Err(e) => {
            info!("读取日志文件失败 {}: {}", file_path, e);
            Vec::new()
        }
    }
}

/// 解析单行日志（格式：YYYY-MM-DD HH:MM:SS LEVEL target - message）
fn parse_log_line(line: &str) -> LogEntryData {
    // 尝试解析 log4rs 格式：timestamp [LEVEL] target - message
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() >= 3 {
        let timestamp = parts[0].to_string();
        // 查找 [LEVEL] 模式
        if let (Some(start), Some(end)) = (line.find('['), line.find(']')) {
            let level = line[start + 1..end].to_string();
            let rest = &line[end + 1..];
            let message = rest.trim_start_matches([' ', '-']).trim().to_string();
            return LogEntryData {
                timestamp,
                level,
                message,
            };
        }
    }
    // 回退：整行作为 message
    LogEntryData {
        timestamp: String::new(),
        level: "INFO".to_string(),
        message: line.to_string(),
    }
}

/// 将 key 字符串解析为 Bevy KeyCode
fn parse_key_code(key: &str) -> Option<KeyCode> {
    Some(match key.to_lowercase().as_str() {
        "space" | " " => KeyCode::Space,
        "enter" | "return" => KeyCode::Enter,
        "escape" | "esc" => KeyCode::Escape,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "arrowup" | "up" => KeyCode::ArrowUp,
        "arrowdown" | "down" => KeyCode::ArrowDown,
        "arrowleft" | "left" => KeyCode::ArrowLeft,
        "arrowright" | "right" => KeyCode::ArrowRight,
        "shift" | "shiftleft" => KeyCode::ShiftLeft,
        "shiftright" => KeyCode::ShiftRight,
        "control" | "ctrl" | "controlleft" => KeyCode::ControlLeft,
        "controlright" => KeyCode::ControlRight,
        "alt" | "altleft" => KeyCode::AltLeft,
        "altright" => KeyCode::AltRight,
        "keya" | "a" => KeyCode::KeyA,
        "keyb" | "b" => KeyCode::KeyB,
        "keyc" | "c" => KeyCode::KeyC,
        "keyd" | "d" => KeyCode::KeyD,
        "keye" | "e" => KeyCode::KeyE,
        "keyf" | "f" => KeyCode::KeyF,
        "keyg" | "g" => KeyCode::KeyG,
        "keyh" | "h" => KeyCode::KeyH,
        "keyi" | "i" => KeyCode::KeyI,
        "keyj" | "j" => KeyCode::KeyJ,
        "keyk" | "k" => KeyCode::KeyK,
        "keyl" | "l" => KeyCode::KeyL,
        "keym" | "m" => KeyCode::KeyM,
        "keyn" | "n" => KeyCode::KeyN,
        "keyo" | "o" => KeyCode::KeyO,
        "keyp" | "p" => KeyCode::KeyP,
        "keyq" | "q" => KeyCode::KeyQ,
        "keyr" | "r" => KeyCode::KeyR,
        "keys" | "s" => KeyCode::KeyS,
        "keyt" | "t" => KeyCode::KeyT,
        "keyu" | "u" => KeyCode::KeyU,
        "keyv" | "v" => KeyCode::KeyV,
        "keyw" | "w" => KeyCode::KeyW,
        "keyx" | "x" => KeyCode::KeyX,
        "keyy" | "y" => KeyCode::KeyY,
        "keyz" | "z" => KeyCode::KeyZ,
        "digit0" | "0" => KeyCode::Digit0,
        "digit1" | "1" => KeyCode::Digit1,
        "digit2" | "2" => KeyCode::Digit2,
        "digit3" | "3" => KeyCode::Digit3,
        "digit4" | "4" => KeyCode::Digit4,
        "digit5" | "5" => KeyCode::Digit5,
        "digit6" | "6" => KeyCode::Digit6,
        "digit7" | "7" => KeyCode::Digit7,
        "digit8" | "8" => KeyCode::Digit8,
        "digit9" | "9" => KeyCode::Digit9,
        "f1" => KeyCode::F1,
        "f2" => KeyCode::F2,
        "f3" => KeyCode::F3,
        "f4" => KeyCode::F4,
        "f5" => KeyCode::F5,
        "f6" => KeyCode::F6,
        "f7" => KeyCode::F7,
        "f8" => KeyCode::F8,
        "f9" => KeyCode::F9,
        "f10" => KeyCode::F10,
        "f11" => KeyCode::F11,
        "f12" => KeyCode::F12,
        _ => return None,
    })
}

/// 在 World 中按 test_id / Name / "bits:{n}" 查找实体
fn find_entity_by_test_id(world: &mut World, id: &str) -> Option<Entity> {
    // 1. 尝试解析 bits 格式
    if let Some(stripped) = id.strip_prefix("bits:") {
        if let Ok(bits) = stripped.parse::<u64>() {
            let entity = Entity::from_bits(bits);
            if world.get_entity(entity).is_ok() {
                return Some(entity);
            }
        }
    }
    // 2. 按 TestId 组件匹配
    let mut query = world.query::<(Entity, &TestId)>();
    for (entity, test_id) in query.iter(world) {
        if test_id.0 == id {
            return Some(entity);
        }
    }
    // 3. 按 Name 组件匹配
    let mut name_query = world.query::<(Entity, &Name)>();
    for (entity, name) in name_query.iter(world) {
        if name.as_str() == id {
            return Some(entity);
        }
    }
    None
}

/// 找到实体并设置 Interaction 组件，返回是否成功
fn find_entity_and_set_interaction(
    world: &mut World,
    id: &str,
    interaction: Interaction,
) -> bool {
    if let Some(entity) = find_entity_by_test_id(world, id) {
        if let Some(mut inter) = world.get_mut::<Interaction>(entity) {
            *inter = interaction;
            info!("设置 Interaction {:?} for entity {:?}", interaction, entity);
            return true;
        }
    }
    info!("未找到可交互元素: {}", id);
    false
}

/// 构建 UI 节点快照（遍历所有带 Node 组件的实体）
fn build_ui_snapshot(world: &mut World) -> Vec<UINodeData> {
    // 收集所有 UI 实体
    let entities: Vec<Entity> = world
        .query_filtered::<Entity, With<Node>>()
        .iter(world)
        .collect();

    let mut nodes = Vec::with_capacity(entities.len());

    for entity in entities {
        let uid = format!("bits:{}", entity.to_bits());

        let name = world
            .get::<Name>(entity)
            .map(|n| n.as_str().to_string())
            .unwrap_or_else(|| uid.clone());

        let test_id = world.get::<TestId>(entity).map(|t| t.0.clone());

        let text = world.get::<Text>(entity).map(|t| t.0.clone());

        let is_button = world.get::<Button>(entity).is_some();
        let node_type = if is_button {
            "button"
        } else if text.is_some() {
            "text"
        } else {
            "container"
        }
        .to_string();

        // Visibility: Hidden = 不可见，Inherited/Visible = 可见
        let visible = world
            .get::<Visibility>(entity)
            .map(|v| *v != Visibility::Hidden)
            .unwrap_or(true);

        // ComputedNode 提供计算后的像素尺寸
        let (width, height) = world
            .get::<ComputedNode>(entity)
            .map(|cn| {
                let s = cn.size();
                (s.x, s.y)
            })
            .unwrap_or((0.0, 0.0));

        // GlobalTransform.translation 给出世界空间位置
        let (x, y) = world
            .get::<GlobalTransform>(entity)
            .map(|gt| {
                let t = gt.translation();
                (t.x, t.y)
            })
            .unwrap_or((0.0, 0.0));

        let parent_uid = world
            .get::<ChildOf>(entity)
            .map(|p| format!("bits:{}", p.parent().to_bits()));

        nodes.push(UINodeData {
            uid,
            name,
            node_type,
            text,
            test_id,
            visible,
            x,
            y,
            width,
            height,
            parent_uid,
        });
    }

    nodes
}
