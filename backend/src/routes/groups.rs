use crate::app_errors::AppError;
use crate::models::{Claims, GroupUser, InvitationState, NewGroup};
use crate::utils::groups::*;
use axum::Router;
use axum::{
    extract::Json,
    routing::{get, post},
    Extension,
};
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::debug;

pub fn router() -> Router {
    Router::new()
        .route("/", get(get_user_groups).post(post_create_group))
        .route("/add-user", post(post_add_user_to_group))
        .layer(Extension(Arc::new(InvitationState::new())))
}

async fn get_user_groups(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Value>, AppError> {
    let res = query_user_groups(&pool, &claims.user_id).await?;

    debug!(
        "Queried user {} ({}) groups successfully",
        &claims.user_id, &claims.login
    );

    Ok(res)
}

async fn post_create_group(
    claims: Claims,
    pool: Extension<PgPool>,
    group: Json<NewGroup>,
) -> Result<(), AppError> {
    tracing::trace!("JWT: {:#?}", claims);
    let res = create_group(&pool, group.name.trim(), claims.user_id).await?;

    debug!("Group {} created successfully", group.name);

    Ok(res)
}

async fn post_add_user_to_group(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    Json(GroupUser { user_id, group_id }): Json<GroupUser>,
) -> Result<(), AppError> {
    tracing::trace!("JWT: {:#?}", claims);
    let res = try_add_user_to_group(&pool, &user_id, &group_id).await?;

    debug!(
        "Added user {} ({}) to group {} successfully",
        &user_id, &claims.login, &group_id
    );

    Ok(res)
}
