use anyhow::Context;
use axum::{Extension, extract};
use sqlx::PgPool;
use crate::queries::{AppError, create_group};
use crate::models::NewGroup;

pub async fn post_create_group(
    pool: Extension<PgPool>,
    group: extract::Json<NewGroup>,
) -> Result<(), AppError> {
    let mut conn = pool.acquire().await.context("Failed to establish connection")?;
    create_group(&mut conn, &group.name).await?;
    Ok(())
}