use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use crossbeam_channel::unbounded;
use log::info;

use crate::test_system::{
    channel::{TestChannel, TEST_COMMAND_CHANNEL},
    graphql::{build_schema, AppSchema},
};

pub fn start_test_server() {
    std::thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let (sender, receiver) = unbounded();

            let _ = TEST_COMMAND_CHANNEL.set(TestChannel {
                sender: sender.clone(),
                receiver,
            });

            let schema = build_schema(sender);

            let app = Router::new()
                .route("/health", get(health_check))
                .route("/graphql", post(graphql_handler))
                .with_state(schema);

            let port = std::env::var("TEST_PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(9222);

            let addr = format!("127.0.0.1:{}", port);
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

            info!("测试自动化服务器启动在 http://{}", addr);

            axum::serve(listener, app).await.unwrap();
        });
    });
}

async fn health_check() -> impl IntoResponse {
    "OK"
}

async fn graphql_handler(
    State(schema): State<AppSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}
