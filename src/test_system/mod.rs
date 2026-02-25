pub mod bevy_systems;
pub mod channel;
pub mod mcp;
pub mod server;

pub use bevy_systems::receive_test_messages;
pub use channel::{TestMessage, TEST_COMMAND_CHANNEL};
pub use server::start_test_server;
