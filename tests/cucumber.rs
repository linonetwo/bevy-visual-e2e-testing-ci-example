use backoff::ExponentialBackoffBuilder;
use cucumber::{given, then, when, StatsWriter, World};
use serde_json::json;
use std::net::TcpStream;
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

mod test_utilities;
use test_utilities::*;

// Backoff 配置工厂函数
fn game_startup_backoff() -> backoff::ExponentialBackoff {
    ExponentialBackoffBuilder::new()
        .with_initial_interval(Duration::from_millis(500))
        .with_max_interval(Duration::from_secs(2))
        .with_max_elapsed_time(Some(Duration::from_secs(10)))
        .build()
}

fn websocket_connection_backoff() -> backoff::ExponentialBackoff {
    ExponentialBackoffBuilder::new()
        .with_initial_interval(Duration::from_millis(100))
        .with_max_interval(Duration::from_secs(1))
        .with_max_elapsed_time(Some(Duration::from_secs(5)))
        .build()
}

fn log_check_backoff() -> backoff::ExponentialBackoff {
    ExponentialBackoffBuilder::new()
        .with_initial_interval(Duration::from_millis(50))
        .with_max_interval(Duration::from_millis(500))
        .with_max_elapsed_time(Some(Duration::from_secs(3)))
        .build()
}

#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct GameWorld {
    log_content: String,
    ws_connection: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    game_process: Option<std::process::Child>,
    test_port: u16,
    log_file_name: String,
    scenario_name: String,
    scenario_dir: String,
}

impl GameWorld {
    fn new() -> Self {
        Self {
            log_content: String::new(),
            ws_connection: None,
            game_process: None,
            test_port: 0,
            log_file_name: String::new(),
            scenario_name: String::new(),
            scenario_dir: String::new(),
        }
    }
}

impl Drop for GameWorld {
    fn drop(&mut self) {
        // 关闭 WebSocket 连接
        if let Some(mut socket) = self.ws_connection.take() {
            let _ = socket.close(None);
        }

        // 停止游戏进程
        if let Some(mut process) = self.game_process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
    }
}

impl GameWorld {
    fn take_screenshot(&mut self, step_name: &str, step_number: usize) {
        let screenshot_path = format!(
            "{}/step_{:02}_{}.png",
            self.scenario_dir, step_number, step_name
        );

        self.send_ws_command("screenshot", json!({ "path": screenshot_path }));
    }

    fn send_hover(&mut self, x: f32, y: f32) {
        self.send_ws_command("hover", json!({ "x": x, "y": y }));
    }

    fn send_click(&mut self, x: f32, y: f32) {
        self.send_ws_command("click", json!({ "x": x, "y": y }));
    }

    // 统一的 WebSocket 命令发送
    fn send_ws_command(&mut self, action: &str, params: serde_json::Value) {
        if self.ws_connection.is_none() {
            self.connect_ws();
        }

        let Some(socket) = &mut self.ws_connection else {
            return;
        };

        let command = json!({
            "action": action,
            "params": params
        });

        if let Err(e) = socket.send(Message::Text(command.to_string().into())) {
            eprintln!("发送{}请求失败: {}", action, e);
            return;
        }

        // 等待响应（现在会等到操作真正完成）
        match socket.read() {
            Ok(msg) => {
                if let Ok(text) = msg.to_text() {
                    if let Ok(response) = serde_json::from_str::<serde_json::Value>(text) {
                        if !response["success"].as_bool().unwrap_or(false) {
                            eprintln!(
                                "{}失败: {}",
                                action,
                                response["message"].as_str().unwrap_or("未知错误")
                            );
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("读取{}响应失败: {}", action, e);
            }
        }
    }

    fn start_game(&mut self, scenario_name: &str) {
        // 为每个场景创建独立的文件夹
        let scenario_dir = format!("logs/{}", scenario_name);
        std::fs::create_dir_all(&scenario_dir).expect("创建场景目录失败");

        // 日志文件放在场景文件夹中
        let log_file_name = format!("{}/game.log", scenario_dir);
        let _ = std::fs::write(&log_file_name, "");

        // 获取项目信息
        let project_name = get_project_name();
        let target_dir = get_target_dir();
        let binary_path = get_binary_path(&project_name, &target_dir);

        // 分配空闲端口
        self.test_port = find_available_port();

        self.log_file_name = log_file_name;
        self.scenario_dir = scenario_dir;

        let child = std::process::Command::new(&binary_path)
            .arg("--test-mode")
            .env("TEST_PORT", self.test_port.to_string())
            .env("TEST_LOG_FILE", &self.log_file_name)
            .spawn()
            .expect("启动游戏失败");

        self.game_process = Some(child);

        // 等待游戏启动 - 使用 backoff 重试连接
        let result = backoff::retry(game_startup_backoff(), || {
            let url = format!("ws://127.0.0.1:{}/ws", self.test_port);
            match tungstenite::connect(&url) {
                Ok((mut socket, _)) => {
                    // 连接成功，关闭测试连接
                    let _ = socket.close(None);
                    Ok(())
                }
                Err(_) => Err(backoff::Error::transient("游戏未就绪")),
            }
        });

        if result.is_err() {
            panic!("游戏启动超时");
        }
    }

    fn get_test_port(&self) -> u16 {
        self.test_port
    }

    fn connect_ws(&mut self) {
        let port = self.get_test_port();
        let url = format!("ws://127.0.0.1:{}/ws", port);

        let result = backoff::retry(
            websocket_connection_backoff(),
            || match tungstenite::connect(&url) {
                Ok((socket, _)) => {
                    self.ws_connection = Some(socket);
                    Ok(())
                }
                Err(e) => Err(backoff::Error::transient(e)),
            },
        );

        if result.is_err() {
            panic!("无法连接到游戏服务器");
        }
    }

    fn read_log(&mut self) {
        self.log_content = read_last_n_lines(&self.log_file_name, 100);
    }
}

#[given("游戏已启动")]
async fn game_is_running(world: &mut GameWorld) {
    let scenario_name = world.scenario_name.clone();
    world.start_game(&scenario_name);
    world.take_screenshot("游戏启动", 1);
}

#[when(expr = "点击按钮 {string}")]
async fn click_button(world: &mut GameWorld, test_id: String) {
    // 根据 test_id 确定点击位置
    let (x, y) = match test_id.as_str() {
        "main-button" => (400.0, 300.0),
        _ => {
            eprintln!("未知的按钮ID: {}", test_id);
            return;
        }
    };

    // 先悬停在按钮上
    world.send_hover(x, y);
    world.take_screenshot("悬停按钮", 2);

    // 然后点击
    world.send_click(x, y);
    world.take_screenshot("点击按钮", 3);
}

#[then(expr = "日志中应该包含 {string}")]
async fn log_should_contain(world: &mut GameWorld, expected: String) {
    let log_file = world.log_file_name.clone();
    let expected_clone = expected.clone();

    let result = backoff::retry(log_check_backoff(), || {
        let content = read_last_n_lines(&log_file, 100);
        if content.contains(&expected_clone) {
            world.log_content = content;
            Ok(())
        } else {
            Err(backoff::Error::transient("日志内容未找到"))
        }
    });

    if result.is_err() {
        // 失败时显示详细日志
        world.read_log();
        let lines: Vec<&str> = world.log_content.lines().collect();
        let last_lines = if lines.len() > 10 {
            &lines[lines.len() - 10..]
        } else {
            &lines[..]
        };
        eprintln!("\n日志检查失败: 期望包含 '{}'", expected);
        eprintln!("日志最后 {} 行:", last_lines.len());
        for line in last_lines {
            eprintln!("  {}", line);
        }
        panic!("日志中未找到期望的内容: {}", expected);
    }
    world.take_screenshot("日志检查", 4);
}

#[then(expr = "存在 {int} 个类型为 {string} 的组件")]
async fn component_count_should_be(world: &mut GameWorld, count: usize, component_type: String) {
    // 通过 WebSocket 查询组件数量
    let url = format!("ws://127.0.0.1:{}/ws", world.get_test_port());

    let (mut socket, _) = tungstenite::connect(&url).expect("WebSocket 连接失败");

    // 发送查询命令
    let query_cmd = json!({
        "action": "query_components",
    });

    socket
        .send(Message::Text(query_cmd.to_string().into()))
        .expect("发送查询命令失败");

    // 接收响应
    let response = socket.read().expect("读取响应失败");
    let response_text = response.to_text().expect("响应不是文本");
    let response_json: serde_json::Value =
        serde_json::from_str(response_text).expect("解析响应失败");

    // 从响应中获取组件数量
    let actual_count = response_json["data"][&component_type]
        .as_u64()
        .unwrap_or_else(|| panic!("未找到组件类型: {}", component_type))
        as usize;

    assert_eq!(
        actual_count, count,
        "组件数量不匹配: 期望 {} 个 {}，实际 {} 个",
        count, component_type, actual_count
    );

    socket.close(None).ok();
    world.take_screenshot("组件数量检查", 5);
}

#[tokio::main]
async fn main() {
    let result = GameWorld::cucumber()
        .before(|_feature, _rule, scenario, world| {
            Box::pin(async move {
                // 从场景名称生成日志文件名
                let clean_name: String = scenario
                    .name
                    .chars()
                    .filter_map(|c| {
                        if c.is_alphanumeric() || c == '_' || c == '-' {
                            Some(c)
                        } else if c.is_whitespace() {
                            Some('_')
                        } else {
                            None
                        }
                    })
                    .collect();

                // 截断到30字符
                let scenario_name = if clean_name.len() > 30 {
                    clean_name[..30].to_string()
                } else {
                    clean_name
                };

                world.scenario_name = scenario_name;
            })
        })
        .run("tests/features/")
        .await;

    // 如果有任何测试失败，退出码为 1
    if !result.execution_has_failed() {
        std::process::exit(0);
    } else {
        eprintln!("\n❌ 测试失败！");
        std::process::exit(1);
    }
}
