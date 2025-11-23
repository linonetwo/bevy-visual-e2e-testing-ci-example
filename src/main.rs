use bevy::log::LogPlugin;
use bevy::prelude::*;
use log::info;
use std::env;

mod font_manager;
mod log_setup;
mod test_automation;

fn main() {
    // 初始化日志系统（支持测试和正常模式）
    log_setup::init_logging();

    info!("游戏启动");
    if let Ok(port) = env::var("TEST_PORT") {
        info!("测试端口: {}", port);
    }
    if let Ok(log) = env::var("TEST_LOG_FILE") {
        info!("日志文件: {}", log);
    }

    // 检查是否为测试模式
    let test_mode = env::args().any(|arg| arg == "--test-mode");
    if test_mode {
        info!("测试模式已启用");
        test_automation::start_test_server();
    }

    let mut app = App::new();

    // 禁用 Bevy 的 LogPlugin，使用我们自己的 log4rs
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "简单游戏".to_string(),
                    resolution: (800, 600).into(),
                    ..default()
                }),
                ..default()
            })
            .build()
            .disable::<LogPlugin>(),
    );

    // 从系统加载中文字体作为默认字体
    let font_config = font_manager::FontConfig::default();
    font_manager::load_and_set_default_font(app.world_mut(), &font_config);

    app.add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                receive_test_messages,
                handle_button_interaction.after(receive_test_messages),
                update_button_visuals,
            ),
        )
        .run();
}

// 测试选择器组件
#[derive(Component, Clone)]
pub struct TestId(pub String);

#[derive(Component)]
struct GameButton;

#[derive(Component)]
struct Ball;

// 从消息队列接收测试消息并直接处理
fn receive_test_messages(
    mut commands: Commands,
    mut button_query: Query<&mut Interaction, With<GameButton>>,
    ball_query: Query<&Ball>,
    button_count_query: Query<&GameButton>,
) {
    use test_automation::TEST_COMMAND_CHANNEL;

    if let Some(channel) = TEST_COMMAND_CHANNEL.get() {
        // 非阻塞地接收所有待处理消息
        while let Ok(msg) = channel.receiver.try_recv() {
            match msg {
                test_automation::TestMessage::Hover { x, y, response } => {
                    info!("收到测试悬停消息: ({}, {})", x, y);
                    // 设置为悬停状态
                    for mut interaction in button_query.iter_mut() {
                        *interaction = Interaction::Hovered;
                    }
                    let _ = response.send(true);
                }
                test_automation::TestMessage::Click { x, y, response } => {
                    info!("收到测试点击消息: ({}, {})", x, y);
                    // 触发按钮按下
                    for mut interaction in button_query.iter_mut() {
                        *interaction = Interaction::Pressed;
                    }
                    let _ = response.send(true);
                }
                test_automation::TestMessage::Screenshot { path, response } => {
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
                test_automation::TestMessage::QueryComponents { response } => {
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

// 处理按钮交互（点击时生成小球）
fn handle_button_interaction(
    interaction: Query<&Interaction, (Changed<Interaction>, With<GameButton>)>,
    mut commands: Commands,
) {
    for interaction in interaction.iter() {
        if *interaction == Interaction::Pressed {
            info!("test-id-button-clicked: main-button");
            info!("按钮被点击!");

            // 生成随机位置的小球
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let x = rng.gen_range(-300.0..300.0);
            let y = rng.gen_range(-200.0..200.0);

            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(400.0 + x),
                    top: Val::Px(300.0 + y),
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::all(Color::srgb(1.0, 1.0, 1.0)),
                BackgroundColor(Color::srgb(1.0, 0.3, 0.3)),
                BorderRadius::all(Val::Percent(50.0)),
                Ball,
                TestId("ball".to_string()),
            ));

            info!("生成小球在位置: ({}, {})", x, y);
        }
    }
}

// 更新按钮视觉状态（响应 Interaction 变化）
#[allow(clippy::type_complexity)]
fn update_button_visuals(
    mut button_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<GameButton>),
    >,
) {
    for (interaction, mut color) in button_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgb(0.3, 0.5, 0.7));
                info!("按钮视觉: 按下状态");
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.5, 0.7, 0.9));
                info!("按钮视觉: 悬停状态");
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.4, 0.6, 0.8));
            }
        }
    }
}

fn setup(mut commands: Commands) {
    // 相机
    commands.spawn(Camera2d);

    // 根节点 - 居中容器
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            // 按钮 - 使用 observe 监听点击事件
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(80.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.2, 0.2, 0.2)),
                    BackgroundColor(Color::srgb(0.4, 0.6, 0.8)),
                    GameButton,
                    TestId("main-button".to_string()),
                    Interaction::None,
                ))
                // 视觉反馈：按下变暗
                .observe(
                    |_trigger: On<Pointer<Press>>,
                     mut query: Query<&mut BackgroundColor, With<GameButton>>| {
                        if let Ok(mut color) = query.single_mut() {
                            *color = BackgroundColor(Color::srgb(0.3, 0.5, 0.7));
                        }
                    },
                )
                // 视觉反馈：释放恢复
                .observe(
                    |_trigger: On<Pointer<Release>>,
                     mut query: Query<&mut BackgroundColor, With<GameButton>>| {
                        if let Ok(mut color) = query.single_mut() {
                            *color = BackgroundColor(Color::srgb(0.5, 0.7, 0.9));
                        }
                    },
                )
                // 视觉反馈：悬停高亮
                .observe(
                    |_trigger: On<Pointer<Over>>,
                     mut query: Query<&mut BackgroundColor, With<GameButton>>| {
                        if let Ok(mut color) = query.single_mut() {
                            *color = BackgroundColor(Color::srgb(0.5, 0.7, 0.9));
                        }
                    },
                )
                // 视觉反馈：离开恢复
                .observe(
                    |_trigger: On<Pointer<Out>>,
                     mut query: Query<&mut BackgroundColor, With<GameButton>>| {
                        if let Ok(mut color) = query.single_mut() {
                            *color = BackgroundColor(Color::srgb(0.4, 0.6, 0.8));
                        }
                    },
                )
                .with_children(|parent| {
                    // 按钮文本
                    parent.spawn((
                        Text::new("点击我"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });

    info!("UI 设置完成");
}
