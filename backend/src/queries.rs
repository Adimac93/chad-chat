use anyhow::Context;
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::{pool::PoolConnection, query, query_as, Acquire, Postgres};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Missing one or more fields")]
    MissingField,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
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
    mut conn: PoolConnection<Postgres>,
    name: &str,
    user_id: Uuid,
) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::MissingField);
    }

    let mut transaction = conn.begin().await.context("Failed to create transaction")?;

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
    .context("Failed to add user to group")?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;

    Ok(())
}
