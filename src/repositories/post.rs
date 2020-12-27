use async_trait::async_trait;
use dataloader::{cached::Loader, BatchFn};
use deadpool_postgres::{Client, Pool};
use slog_scope::{error, info};
use std::{collections::HashMap, sync::Arc};
use tokio_pg_mapper::FromTokioPostgresRow;
use tokio_postgres::{error::SqlState, Error};
use uuid::Uuid;

use crate::{
    errors::{AppError, AppErrorType},
    models::post::{CreatePost, Post},
};

pub struct PostRepository {
    pool: Arc<Pool>,
}

pub struct PostBatcher {
    pool: Arc<Pool>,
}

pub type PostLoader = Loader<Uuid, Vec<Post>, AppError, PostBatcher>;

pub fn get_posts_loader(pool: Arc<Pool>) -> PostLoader {
    Loader::new(PostBatcher { pool }).with_yield_count(100)
}

impl PostBatcher {
    pub async fn get_posts_by_users_ids(
        &self,
        hashmap: &mut HashMap<Uuid, Vec<Post>>,
        ids: Vec<Uuid>,
    ) -> Result<(), AppError> {
        let client: Client = self.pool.get().await.map_err(|err| {
            error!("Error getting parsing posts. {}", err; "query" => "get_posts_by_users_ids");
            err
        })?;

        let statement = client
            .prepare("select * from posts where author_id = ANY($1)")
            .await?;

        client
            .query(&statement, &[&ids])
            .await?
            .iter()
            .map(|row| Post::from_row_ref(row))
            .collect::<Result<Vec<Post>, _>>()
            .map_err(|err| {
                error!("Error getting parsing posts. {}", err; "query" => "get_posts_by_users_ids");
                err
            })?
            .iter()
            .fold(hashmap, |map, post| {
                let vec = map
                    .entry(post.author_id)
                    .or_insert_with(|| Vec::<Post>::new());
                vec.push(post.clone());
                map
            });

        Ok(())
    }
}

#[async_trait]
impl BatchFn<Uuid, Vec<Post>> for PostBatcher {
    type Error = AppError;

    async fn load(&self, keys: &[Uuid]) -> HashMap<Uuid, Result<Vec<Post>, AppError>> {
        info!("Loading batch {:?}", keys);

        let mut posts_map: HashMap<Uuid, Vec<Post>> = HashMap::new();

        let result: Result<(), AppError> = self
            .get_posts_by_users_ids(&mut posts_map, keys.into())
            .await;

        keys.iter()
            .map(move |id| {
                let entry = posts_map.entry(*id).or_insert_with(|| vec![]);
                (id.clone(), result.clone().map(|_| entry.clone()))
            })
            .collect::<HashMap<_, _>>()
    }
}

impl PostRepository {
    pub fn new(pool: Arc<Pool>) -> PostRepository {
        PostRepository { pool }
    }

    pub async fn get(&self, id: Uuid) -> Result<Post, AppError> {
        let client: Client = self.pool.get().await.map_err(|err| {
            error!("Error getting parsing users. {}", err; "query" => "get");
            err
        })?;

        let statement = client.prepare("select * from post where id = $1").await?;

        client
            .query(&statement, &[&id])
            .await?
            .iter()
            .map(|row| Post::from_row_ref(row))
            .collect::<Result<Vec<Post>, _>>()?
            .pop()
            .ok_or(AppError {
                cause: None,
                message: Some(format!("Post with id {} not found", id)),
                error_type: AppErrorType::NotFoundError,
            })
    }

    pub async fn all(&self) -> Result<Vec<Post>, AppError> {
        let client: Client = self.pool.get().await.map_err(|err| {
            error!("Error getting parsing posts. {}", err; "query" => "posts");
            err
        })?;

        let statement = client.prepare("select * from posts").await?;

        let posts = client
            .query(&statement, &[])
            .await?
            .iter()
            .map(|row| Post::from_row_ref(row))
            .collect::<Result<Vec<Post>, _>>()
            .map_err(|err| {
                error!("Error getting parsing posts. {}", err; "query" => "posts");
                err
            })?;

        Ok(posts)
    }

    pub async fn create(&self, input: CreatePost) -> Result<Post, AppError> {
        let client: Client = self.pool.get().await.map_err(|err| {
            error!("Error getting parsing posts. {}", err; "query" => "create post");
            err
        })?;

        let statement = client
        .prepare("insert into posts (author_id, slug, title, description, body) values ($1, $2, $3, $4, $5) returning *")
        .await?;

        let slug = match input.slug {
            Some(s) => s,
            None => Uuid::new_v4().to_string(),
        };

        let author_id = input.author_id.clone();

        let post = client
            .query(
                &statement,
                &[
                    &input.author_id,
                    &slug,
                    &input.title,
                    &input.description,
                    &input.body,
                ],
            )
            .await
            .map_err(|err: Error| match err.code() {
                Some(code) => match code {
                    c if c == &SqlState::UNIQUE_VIOLATION => AppError {
                        cause: Some(err.to_string()),
                        message: Some(format!("Slug {} already exists", slug)),
                        error_type: AppErrorType::InvalidField,
                    },
                    c if c == &SqlState::FOREIGN_KEY_VIOLATION => AppError {
                        cause: Some(err.to_string()),
                        message: Some(format!("Author with id {} does not exists", author_id)),
                        error_type: AppErrorType::InvalidField,
                    },
                    _ => AppError::from(err),
                },
                _ => AppError::from(err),
            })?
            .iter()
            .map(|row| Post::from_row_ref(row))
            .collect::<Result<Vec<Post>, _>>()?
            .pop()
            .ok_or(AppError {
                message: Some("Error creating Post.".to_string()),
                cause: None,
                error_type: AppErrorType::DbError,
            })?;

        Ok(post)
    }
}
