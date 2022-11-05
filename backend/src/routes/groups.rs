use crate::models::{Claims, NewGroup};
use crate::queries::{create_group, AppError};
use anyhow::Context;
use axum::{extract, Extension};
use sqlx::PgPool;

pub async fn post_create_group(
    claims: Claims,
    pool: Extension<PgPool>,
    group: extract::Json<NewGroup>,
) -> Result<(), AppError> {
    tracing::trace!("JWT: {:#?}", claims);
    let conn = pool
        .acquire()
        .await
        .context("Failed to establish connection")?;
    create_group(conn, group.name.trim()).await?;
    Ok(())
}
