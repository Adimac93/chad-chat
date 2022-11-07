use crate::models::{Claims, NewMessage};
use crate::queries::{AppError, create_message};
use anyhow::Context;
use axum::{extract, Extension};
use sqlx::PgPool;

pub async fn post_create_message(
    claims: Claims,
    pool: Extension<PgPool>,
    message: extract::Json<NewMessage>,
) -> Result<(), AppError> {
    tracing::trace!("JWT: {:#?}", claims);
    let conn = pool
        .acquire()
        .await
        .context("Failed to establish connection")?;
    
    create_message(conn, message.user_id, message.group_id, message.content.trim()).await
}