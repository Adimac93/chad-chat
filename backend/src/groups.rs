use anyhow::Context;
use axum::{response::IntoResponse, http::status::StatusCode, Json};
use serde_json::json;
use sqlx::{query, Postgres, Pool};
use thiserror::Error;
use uuid::Uuid;


#[derive(Error, Debug)]
pub enum GroupError {
    #[error("Already in group")]
    UserAlreadyInGroup,
    #[error("Wrong invitation url")]
    BadInvitation,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error)
}

impl IntoResponse for GroupError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            GroupError::UserAlreadyInGroup => StatusCode::BAD_REQUEST,
            GroupError::BadInvitation => StatusCode::BAD_REQUEST,
            GroupError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            },
        };

        let info = match self {
            GroupError::Unexpected(_) => "Unexpected server error".into(),
            _ => format!("{self:?}")
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}

pub async fn try_add_user_to_group(pool: &Pool<Postgres>, user_id: &Uuid, group_id: &Uuid) -> Result<(), GroupError> {
    // cannot tell a difference between finding an already existing user and any other kind of error
    // if (user_id, group_id) was a composite primary key
    let mut transaction = pool.begin().await.context("Failed to begin transaction")?;
    
    let res = query!(
    r#"
        select * from group_users 
        where user_id = $1 and group_id = $2
    "#,
    user_id,
    group_id
    ).fetch_one(&mut transaction)
    .await
    .context("Failed to select group user")?;
    
    if &res.user_id == user_id{
        transaction.rollback().await.context("Failed when aborting transaction")?;
        return Err(GroupError::UserAlreadyInGroup)
    }

    query!(
        r#"
            insert into group_users (user_id, group_id)
            values ($1, $2)
        "#,
        user_id,
        group_id
    )
    .execute(&mut transaction)
    .await
    .context("Failed to add user to group")?;

    transaction.commit().await.context("Transaction failed")?;
    
    Ok(())
}