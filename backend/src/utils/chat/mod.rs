pub mod messages;
pub mod models;
pub mod socket;

use crate::errors::DbErrMessage;
use anyhow::Context;
use futures::TryFutureExt;
use hyper::StatusCode;
use sqlx::{query, PgPool};
use uuid::Uuid;

use crate::errors::AppError;

pub async fn get_group_nickname(
    pool: &PgPool,
    user_id: &Uuid,
    group_id: &Uuid,
) -> Result<String, AppError> {
    let res = query!(
        r#"
            SELECT nickname FROM group_users
            WHERE user_id = $1 AND group_id = $2
        "#,
        user_id,
        group_id
    )
    .fetch_one(pool)
    .await
    .context("Cannot fetch user nickname from database")?;

    Ok(res.nickname)
}

pub async fn get_user_email_by_id(pool: &PgPool, user_id: &Uuid) -> sqlx::Result<String> {
    let res = query!(
        r#"
            select email from credentials where id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(res.email)
}

pub async fn create_message(
    pool: &PgPool,
    user_id: &Uuid,
    group_id: &Uuid,
    content: &str,
) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Err(AppError::exp(StatusCode::BAD_REQUEST, "Empty message"));
    }

    insert_message(pool, user_id, group_id, content)
        .map_err(|e| {
            DbErrMessage::new(e).fk(StatusCode::BAD_REQUEST, "Group or user does not exist")
        })
        .await?;

    Ok(())
}

async fn insert_message(
    pool: &PgPool,
    user_id: &Uuid,
    group_id: &Uuid,
    content: &str,
) -> sqlx::Result<()> {
    query!(
        r#"
            INSERT INTO messages (content, user_id, group_id)
            VALUES ($1, $2, $3)
        "#,
        content,
        user_id,
        group_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod test {
    use sqlx::{query, PgPool};
    #[sqlx::test]
    async fn select_user_groups(pool: PgPool) {
        let name = String::from("abc");
        let _res = query!(
            r#"
                INSERT INTO groups (name)
                VALUES ($1)
                RETURNING id
            "#,
            name
        )
        .fetch_one(&pool)
        .await
        .unwrap();
    }
}
