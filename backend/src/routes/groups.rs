use crate::models::{Claims, NewGroup, GroupUser};
use crate::queries::{create_group, AppError, try_add_user_to_group};
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
    
    create_group(conn, group.name.trim(),claims.id).await
}

pub async fn post_add_user_to_group(
    claims: Claims,
    pool: Extension<PgPool>,
    axum::Json(GroupUser{user_id, group_id}): extract::Json<GroupUser>,
) -> Result<(), AppError> {
    tracing::trace!("JWT: {:#?}", claims);
    let conn = pool
        .acquire()
        .await
        .context("Failed to establish connection")?;
    
    try_add_user_to_group(conn, user_id, group_id).await
}