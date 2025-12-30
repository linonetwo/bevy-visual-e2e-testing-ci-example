use backoff::ExponentialBackoffBuilder;
use cucumber::{given, then, when, StatsWriter, World};
use serde_json::json;
use std::time::Duration;

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
    http_client: reqwest::Client,
    game_process: Option<std::process::Child>,
    test_port: u16,
    base_url: String,
    log_file_name: String,
    scenario_name: String,
    scenario_dir: String,
}

impl GameWorld {
    fn new() -> Self {
        Self {
            log_content: String::new(),
            http_client: reqwest::Client::new(),
            game_process: None,
            test_port: 0,
            base_url: String::new(),
            log_file_name: String::new(),
            scenario_name: String::new(),
            scenario_dir: String::new(),
        }
    }
}

impl Drop for GameWorld {
    fn drop(&mut self) {
        // 停止游戏进程
        if let Some(mut process) = self.game_process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
    }
}

impl GameWorld {
    async fn take_screenshot(&mut self, step_name: &str, step_number: usize) {
        let screenshot_path = format!(
            "{}/step_{:02}_{}.png",
            self.scenario_dir, step_number, step_name
        );

        self.screenshot(&screenshot_path).await;
    }

    fn graphql_endpoint(&self) -> String {
        format!("{}/graphql", self.base_url)
    }

    fn health_endpoint(&self) -> String {
        format!("{}/health", self.base_url)
    }

    async fn graphql_request(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let payload = json!({
            "query": query,
            "variables": variables,
        });

        let response = self
            .http_client
            .post(self.graphql_endpoint())
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("发送 GraphQL 请求失败: {}", e))?;

        let status = response.status();
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("解析 GraphQL 响应失败: {}", e))?;

        if !status.is_success() {
            return Err(format!("GraphQL HTTP 状态异常: {}，响应: {}", status, value));
        }

        if let Some(errors) = value.get("errors") {
            return Err(format!("GraphQL errors: {}", errors));
        }

        Ok(value)
    }

    async fn hover(&self, x: f32, y: f32) {
        let query = r#"
            mutation Hover($x: Float!, $y: Float!) {
              hover(x: $x, y: $y) { success message }
            }
        "#;

        match self.graphql_request(query, json!({"x": x, "y": y})).await {
            Ok(resp) => {
                let ok = resp["data"]["hover"]["success"].as_bool().unwrap_or(false);
                if !ok {
                    eprintln!(
                        "hover 失败: {}",
                        resp["data"]["hover"]["message"].as_str().unwrap_or("未知错误")
                    );
                }
            }
            Err(e) => eprintln!("hover 请求失败: {}", e),
        }
    }

    async fn click(&self, x: f32, y: f32) {
        let query = r#"
            mutation Click($x: Float!, $y: Float!) {
              click(x: $x, y: $y) { success message }
            }
        "#;

        match self.graphql_request(query, json!({"x": x, "y": y})).await {
            Ok(resp) => {
                let ok = resp["data"]["click"]["success"].as_bool().unwrap_or(false);
                if !ok {
                    eprintln!(
                        "click 失败: {}",
                        resp["data"]["click"]["message"].as_str().unwrap_or("未知错误")
                    );
                }
            }
            Err(e) => eprintln!("click 请求失败: {}", e),
        }
    }

    async fn screenshot(&self, path: &str) {
        let query = r#"
            mutation Screenshot($path: String!) {
              screenshot(path: $path) { success message }
            }
        "#;

        match self.graphql_request(query, json!({"path": path})).await {
            Ok(resp) => {
                let ok = resp["data"]["screenshot"]["success"].as_bool().unwrap_or(false);
                if !ok {
                    eprintln!(
                        "screenshot 失败: {}",
                        resp["data"]["screenshot"]["message"].as_str().unwrap_or("未知错误")
                    );
                }
            }
            Err(e) => eprintln!("screenshot 请求失败: {}", e),
        }
    }

    async fn start_game(&mut self, scenario_name: &str) {
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
        self.base_url = format!("http://127.0.0.1:{}", self.test_port);

        self.log_file_name = log_file_name;
        self.scenario_dir = scenario_dir;

        let child = std::process::Command::new(&binary_path)
            .arg("--test-mode")
            .env("TEST_PORT", self.test_port.to_string())
            .env("TEST_LOG_FILE", &self.log_file_name)
            .spawn()
            .expect("启动游戏失败");

        self.game_process = Some(child);

        let mut backoff = game_startup_backoff();
        let mut ok = false;
        while let Some(wait) = backoff::backoff::Backoff::next_backoff(&mut backoff) {
            let resp = self.http_client.get(self.health_endpoint()).send().await;
            if matches!(resp, Ok(r) if r.status().is_success()) {
                ok = true;
                break;
            }
            tokio::time::sleep(wait).await;
        }

        if !ok {
              panic!("游戏启动超时。\n\n请参考 .github/workflows/test.yml，安装 Linux 依赖，并用 xvfb-run 运行测试：\n\nsudo apt-get install ...（依赖列表见 test.yml）\nxvfb-run --auto-servernum --server-args=\"-screen 0 1024x768x24\" cargo test\n");
        }
    }

    fn read_log(&mut self) {
        self.log_content = read_last_n_lines(&self.log_file_name, 100);
    }
}

#[given("游戏已启动")]
async fn game_is_running(world: &mut GameWorld) {
    let scenario_name = world.scenario_name.clone();
    world.start_game(&scenario_name).await;
    world.take_screenshot("游戏启动", 1).await;
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
    world.hover(x, y).await;
    world.take_screenshot("悬停按钮", 2).await;

    // 然后点击
    world.click(x, y).await;
    world.take_screenshot("点击按钮", 3).await;
}

#[then(expr = "日志中应该包含 {string}")]
async fn log_should_contain(world: &mut GameWorld, expected: String) {
    let log_file = world.log_file_name.clone();
    let expected_clone = expected.clone();

    let mut backoff = log_check_backoff();
    let mut ok = false;
    while let Some(wait) = backoff::backoff::Backoff::next_backoff(&mut backoff) {
        let content = read_last_n_lines(&log_file, 100);
        if content.contains(&expected_clone) {
            world.log_content = content;
            ok = true;
            break;
        }
        tokio::time::sleep(wait).await;
    }

    if !ok {
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
    world.take_screenshot("日志检查", 4).await;
}

#[then(expr = "存在 {int} 个类型为 {string} 的组件")]
async fn component_count_should_be(world: &mut GameWorld, count: usize, component_type: String) {
    let query = r#"
        query ComponentCounts {
          componentCounts { name count }
        }
    "#;

    let response_json = world
        .graphql_request(query, json!({}))
        .await
        .expect("GraphQL 查询失败");

    let counts = response_json["data"]["componentCounts"]
        .as_array()
        .expect("componentCounts 不是数组");

    let actual_count = counts
        .iter()
        .find(|v| v["name"].as_str() == Some(component_type.as_str()))
        .and_then(|v| v["count"].as_i64())
        .unwrap_or_else(|| panic!("未找到组件类型: {}", component_type)) as usize;

    assert_eq!(
        actual_count, count,
        "组件数量不匹配: 期望 {} 个 {}，实际 {} 个",
        count, component_type, actual_count
    );

    world.take_screenshot("组件数量检查", 5).await;
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
