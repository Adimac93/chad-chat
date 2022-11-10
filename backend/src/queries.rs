use anyhow::Context;
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::{pool::PoolConnection, query, query_as, Acquire, Postgres, Connection, Pool};
use thiserror::Error;
use uuid::Uuid;

use crate::groups::GroupError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("This user is already in this group")]
    GroupUserAlreadyExists,
    #[error("Missing one or more fields")]
    MissingField,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            AppError::GroupUserAlreadyExists => StatusCode::BAD_REQUEST,
            AppError::MissingField => StatusCode::BAD_REQUEST,
            AppError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let info = match self {
            AppError::Unexpected(_) => "Unexpected server error".into(),
            _ => format!("{self:?}"),
        };

        (status, Json(json!({ "error_info": info }))).into_response()
    }
}

struct Group {
    pub id: Uuid,
    pub name: String,
}

pub async fn create_group(
    pool: &Pool<Postgres>,
    name: &str,
    user_id: Uuid,
) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::MissingField);
    }

    let mut transaction = pool.begin().await.context("Failed to create transaction")?;

    let group = query_as!(
        Group,
        r#"
            insert into groups (name)
            values ($1)
            returning *
        "#,
        name
    )
    .fetch_one(&mut transaction)
    .await
    .context("Failed to create a group")?;

    query!(
        r#"
            insert into group_users (user_id, group_id)
            values ($1, $2)
        "#,
        user_id,
        group.id
    )
    .execute(&mut transaction)
    .await
    .map_err(|_| AppError::GroupUserAlreadyExists)?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;

    Ok(())
}


pub async fn create_message(pool: &Pool<Postgres>, user_id: &Uuid, group_id: &Uuid, content: &str) -> Result<(), AppError> {
    query!(
        r#"
            insert into messages (content, user_id, group_id)
            values ($1, $2, $3)
        "#,
        content,
        user_id,
        group_id
    )
    .execute(pool)
    .await
    .context("Failed to add message")?;
    Ok(())
}

pub async fn check_if_group_member(pool: &Pool<Postgres>, user_id: &Uuid, group_id: &Uuid) -> Result<bool, AppError> {
    let res = query!(
        r#"
            select * from group_users
            where user_id = $1 and group_id = $2
        "#,
        user_id,
        group_id
    )
    .fetch_optional(pool)
    .await
    .context("Failed to check if user is in group")?;

    Ok(res.is_some())
}