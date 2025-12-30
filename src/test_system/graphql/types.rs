use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone, Debug)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct ComponentCount {
    pub name: String,
    pub count: i32,
}
