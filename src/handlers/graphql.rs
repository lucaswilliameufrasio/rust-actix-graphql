use crate::models::user::User;
use deadpool_postgres::{Client, Pool};
use juniper::{EmptyMutation, EmptySubscription, FieldError, RootNode};
use std::sync::Arc;
use tokio_pg_mapper::FromTokioPostgresRow;

#[derive(Clone)]
pub struct Context {
    pub pool: Arc<Pool>,
}

impl juniper::Context for Context {}

pub struct Query {}

#[juniper::graphql_object(
    Context = Context
)]
impl Query {
    pub async fn ApiVersion() -> &str {
        "1.0"
    }

    pub async fn users(context: &Context) -> Result<Vec<User>, FieldError> {
        let client: Client = context.pool.get().await?;

        let statement = client.prepare("select * from users").await?;

        let users = client
            .query(&statement, &[])
            .await?
            .iter()
            .map(|row| User::from_row_ref(row))
            .collect::<Result<Vec<User>, _>>()?;

        Ok(users)
    }
}

pub type Schema = RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(
        Query {},
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    )
}
