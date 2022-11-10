use crate::models::{Claims, Message};
use crate::queries::{AppError, create_message};
use anyhow::Context;
use axum::{extract, Extension};
use sqlx::PgPool;

pub async fn post_create_message(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    message: extract::Json<Message>,
) -> Result<(), AppError> {
    tracing::trace!("JWT: {:#?}", claims);
    create_message(&pool, &message.user_id, &message.group_id, message.content.trim()).await
}