use anyhow::Context;
use sqlx::{query_as, PgPool};
use uuid::Uuid;

use super::{errors::ChatError, models::MessageModel};

pub async fn fetch_last_messages_in_range(
    pool: &PgPool,
    group_id: &Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<MessageModel>, ChatError> {
    let mut messages = query_as!(
        MessageModel,
        r#"
            select * from messages
            where group_id = $1
            order by id desc
            limit $2 offset $3
        "#,
        group_id,
        limit,
        offset
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch last messages")?;

    messages.reverse();
    Ok(messages)
}

pub async fn fetch_all_messages(
    pool: &PgPool,
    group_id: &Uuid,
) -> Result<Vec<MessageModel>, ChatError> {
    let messages = query_as!(
        MessageModel,
        r#"
            select * from messages
            where group_id = $1
        "#,
        group_id
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch last messages")?;

    Ok(messages)
}
