use super::{mutations::MutationRoot, queries::QueryRoot};
use async_graphql::{EmptySubscription, Schema};
use crossbeam_channel::Sender;

use crate::test_system::channel::TestMessage;

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(sender: Sender<TestMessage>) -> AppSchema {
    Schema::build(QueryRoot::default(), MutationRoot::default(), EmptySubscription)
        .data(sender)
        .finish()
}
