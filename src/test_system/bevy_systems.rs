use bevy::prelude::*;
use log::info;

use crate::{Ball, GameButton};
use crate::test_system::{TestMessage, TEST_COMMAND_CHANNEL};

// 从消息队列接收测试消息并直接处理
pub fn receive_test_messages(
    mut commands: Commands,
    mut button_query: Query<&mut Interaction, With<GameButton>>,
    ball_query: Query<&Ball>,
    button_count_query: Query<&GameButton>,
) {
    if let Some(channel) = TEST_COMMAND_CHANNEL.get() {
        // 非阻塞地接收所有待处理消息
        while let Ok(msg) = channel.receiver.try_recv() {
            match msg {
                TestMessage::Hover { x, y, response } => {
                    info!("收到测试悬停消息: ({}, {})", x, y);
                    // 设置为悬停状态
                    for mut interaction in button_query.iter_mut() {
                        *interaction = Interaction::Hovered;
                    }
                    let _ = response.send(true);
                }
                TestMessage::Click { x, y, response } => {
                    info!("收到测试点击消息: ({}, {})", x, y);
                    // 触发按钮按下
                    for mut interaction in button_query.iter_mut() {
                        *interaction = Interaction::Pressed;
                    }
                    let _ = response.send(true);
                }
                TestMessage::Screenshot { path, response } => {
                    info!("收到截图请求: {}", path);
                    let path_clone = path.clone();

                    // 发送截图命令
                    commands
                        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                        .observe(bevy::render::view::screenshot::save_to_disk(path));

                    // 在后台线程用 backoff 等待文件生成
                    std::thread::spawn(move || {
                        use backoff::ExponentialBackoffBuilder;
                        use std::time::Duration;

                        let backoff_config = ExponentialBackoffBuilder::new()
                            .with_initial_interval(Duration::from_millis(50))
                            .with_max_interval(Duration::from_millis(500))
                            .with_max_elapsed_time(Some(Duration::from_secs(5)))
                            .build();

                        let result = backoff::retry(backoff_config, || {
                            if std::path::Path::new(&path_clone).exists() {
                                Ok(())
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

                    // 发送响应
                    let _ = response.send(counts);
                }
            }
        }
    }
}
