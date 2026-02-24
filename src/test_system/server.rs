use axum::{response::IntoResponse, routing::{get, post}, Router};
use crossbeam_channel::unbounded;
use log::info;

use crate::test_system::{
    channel::{TestChannel, TEST_COMMAND_CHANNEL},
    mcp::mcp_handler,
};

pub fn start_test_server() {
    std::thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let (sender, receiver) = unbounded();
            let _ = TEST_COMMAND_CHANNEL.set(TestChannel { sender: sender.clone(), receiver });

            let app = Router::new()
                .route("/health", get(health_check))
                .route("/mcp",    post(mcp_handler))
                .with_state(sender);

            let port = std::env::var("TEST_PORT")
                .ok().and_then(|p| p.parse::<u16>().ok()).unwrap_or(9222);
            let addr = format!("127.0.0.1:{}", port);
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

            info!("测试服务器: http://{}",      addr);
            info!("MCP:        http://{}/mcp", addr);
            axum::serve(listener, app).await.unwrap();
        });
    });
}

async fn health_check() -> impl IntoResponse { "OK" }
