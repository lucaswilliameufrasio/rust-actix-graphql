use deadpool_postgres::{Client, Pool};
use slog_scope::error;
use tokio_postgres::{Error, error::SqlState};
use uuid::Uuid;
use std::sync::Arc;
use tokio_pg_mapper::FromTokioPostgresRow;

use crate::{config::HashingService, errors::{AppError, AppErrorType}, models::user::{CreateUser, User}};

pub struct UserRepository {
    pool: Arc<Pool>,
}

impl UserRepository {
    pub fn new(pool: Arc<Pool>) -> UserRepository {
        UserRepository { pool }
    }

    pub async fn get(&self, id: Uuid) -> Result<User, AppError> {
        let client: Client = self.pool
        .get()
        .await
        .map_err(|err| {
            error!("Error getting parsing users. {}", err; "query" => "get");
            err
        })?;

        let statement = client.prepare("select * from users where id = $1").await?;

        client
            .query(&statement, &[&id])
            .await?
            .iter()
            .map(|row| User::from_row_ref(row))
            .collect::<Result<Vec<User>, _>>()?
            .pop()
            .ok_or(AppError {
                cause: None,
                message: Some(format!("User with id {} not found", id)),
                error_type: AppErrorType::NotFoundError
            })
    }

    pub async fn all(&self) -> Result<Vec<User>, AppError>  {
        let client: Client = self.pool
        .get()
        .await
        .map_err(|err| {
            error!("Error getting parsing users. {}", err; "query" => "users");
            err
        })?;

        let statement = client.prepare("select * from users").await?;

        let users = client
            .query(&statement, &[])
            .await?
            .iter()
            .map(|row| User::from_row_ref(row))
            .collect::<Result<Vec<User>, _>>()
            .map_err(|err| {
                error!("Error getting parsing users. {}", err; "query" => "users");
                err
            })?;

        Ok(users)
    }

    pub async fn create(&self, input: CreateUser, hashing: Arc<HashingService>) -> Result<User, AppError> {
        let client: Client = self.pool
        .get()
        .await
        .map_err(|err| {
            error!("Error getting parsing users. {}", err; "query" => "create_user");
            err
        })?;

        let statement = client
        .prepare("insert into users (username, email, password, bio, image) values ($1, $2, $3, $4, $5) returning *")
        .await?;

        let password_hash = hashing.hash(input.password).await?;

        let user = client
            .query(&statement, &[
                &input.username,
                &input.email,
                &password_hash,
                &input.bio,
                &input.image,
            ])
            .await
            .map_err(|err: Error| {
                let unique_error = err.code()
                    .map(|code: &SqlState| code == &SqlState::UNIQUE_VIOLATION);
                
                    match unique_error {
                        Some(true) => AppError {
                            cause: Some(err.to_string()),
                            message: Some("Username or email address already in use.".to_string()),
                            error_type: AppErrorType::InvalidField
                        },
                        _ => AppError::from(err)
                    }
            })?
            .iter()
            .map(|row| User::from_row_ref(row))
            .collect::<Result<Vec<User>, _>>()?
            .pop()
            .ok_or(AppError {
                message: Some("Error creating User.".to_string()),
                cause: None,
                error_type: AppErrorType::DbError
            })?;

        Ok(user)
    }
}