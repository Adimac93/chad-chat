use anyhow::Context;
use sqlx::{query_as, PgPool};
use uuid::Uuid;

use super::models::{GroupUserMessage, GroupUserMessageModel};

use crate::errors::AppError;

pub async fn fetch_last_messages_in_range(
    pool: &PgPool,
    group_id: &Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<GroupUserMessage>, AppError> {
    let messages = query_as!(
        GroupUserMessageModel,
        r#"
            SELECT gu.nickname, m.content, m.sent_at from messages as m
            JOIN group_users gu ON m.group_id = gu.group_id
            WHERE m.group_id = $1
            ORDER BY id DESC
            LIMIT $2 OFFSET $3
        "#,
        group_id,
        limit,
        offset
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch last messages")?;

    let messages = messages
        .into_iter()
        .rev()
        .map(|msg| GroupUserMessage {
            content: msg.content,
            nickname: msg.nickname,
            sat: msg.sent_at.unix_timestamp(),
        })
        .rev()
        .collect();

    Ok(messages)
}
