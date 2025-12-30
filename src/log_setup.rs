use log::LevelFilter;
use std::env;

/// 初始化日志系统
///
/// 支持两种模式：
/// - 测试模式：根据 TEST_LOG_FILE 环境变量配置独立日志文件
/// - 正常模式：使用 log4rs.yaml 配置文件
pub fn init_logging() {
    let log_file = env::var("TEST_LOG_FILE").unwrap_or_else(|_| "logs/game.log".to_string());

    if env::var("TEST_LOG_FILE").is_ok() {
        setup_test_logging(&log_file);
    } else {
        setup_normal_logging();
    }
}

/// 测试模式日志配置
///
/// 创建两个日志文件：
/// - 主日志：只记录业务日志（2KB 左右）
/// - Debug 日志：根据 TEST_DEBUG 环境变量决定级别
///   - 默认 INFO：约 28KB（包含所有库的 INFO 日志）
///   - TEST_DEBUG=1：约 7MB（包含 Vulkan 等所有调试信息）
fn setup_test_logging(log_file: &str) {
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Logger, Root};
    use log4rs::encode::pattern::PatternEncoder;
    use log4rs::filter::threshold::ThresholdFilter;

    // 主日志文件 - 只记录我们自己的 INFO（过滤掉第三方库）
    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} [{l}] {m}{n}",
        )))
        .build(log_file)
        .expect("Failed to create file appender");

    // Debug 日志文件 - 默认 INFO，可通过 TEST_DEBUG=1 开启完整调试
    let debug_level = if env::var("TEST_DEBUG").is_ok() {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    let debug_file = log_file.replace(".log", ".debug.log");
    let debug_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} [{l}] {t} - {m}{n}",
        )))
        .build(&debug_file)
        .expect("Failed to create debug file appender");

    let config = log4rs::config::Config::builder()
        // 主日志 appender：只有 INFO 及以上
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
                .build("file", Box::new(file_appender)),
        )
        // Debug 日志 appender：根据环境变量决定级别
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(debug_level)))
                .build("debug_file", Box::new(debug_appender)),
        )
        // 我们的应用日志：输出到主日志
        .logger(
            Logger::builder()
                .appender("file")
                .appender("debug_file")
                .additive(false)
                .build("simple_game", LevelFilter::Info),
        )
        // 测试系统日志：输出到主日志
        .logger(
            Logger::builder()
                .appender("file")
                .appender("debug_file")
                .additive(false)
            .build("test_system", LevelFilter::Info),
        )
        // 其他所有库：只输出到 debug 日志
        .build(Root::builder().appender("debug_file").build(debug_level))
        .expect("Failed to build log config");

    log4rs::init_config(config).expect("Failed to initialize log4rs");
}

/// 正常模式日志配置
fn setup_normal_logging() {
    log4rs::init_file("log4rs.yaml", Default::default()).expect("Failed to initialize log4rs");
}
