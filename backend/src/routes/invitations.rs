use crate::models::{Claims, GroupInfo, GroupUser, InvitationState, NewGroup, NewGroupInvitation};
use crate::utils::groups::errors::*;
use crate::utils::groups::*;
use crate::utils::invitations::errors::InvitationError;
use crate::utils::invitations::{
    fetch_group_info_by_code, try_create_group_invitation_with_code, try_join_group_by_code,
    GroupInvitationCreate,
};
use anyhow::Context;
use axum::response::IntoResponse;
use axum::Router;
use axum::{
    extract::{Json, Path},
    routing::{get, post},
    Extension,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub fn router() -> Router {
    Router::new()
        .route("/info", post(post_fetch_group_info_by_code))
        .route("/create", post(post_generate_group_invitation_code))
        .route("/join", post(post_join_group_by_code))
}

async fn post_generate_group_invitation_code(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    Json(invitation): Json<GroupInvitationCreate>,
) -> Result<Json<Value>, InvitationError> {
    let invitation =
        try_create_group_invitation_with_code(&pool, &claims.user_id, invitation).await?;
    Ok(Json(json!({ "code": invitation })))
}

#[derive(Serialize, Deserialize)]
struct JoinGroupCode {
    code: String,
}

async fn post_fetch_group_info_by_code(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<JoinGroupCode>,
) -> Result<Json<GroupInfo>, InvitationError> {
    Ok(Json(
        fetch_group_info_by_code(&pool, &claims.user_id, &payload.code).await?,
    ))
}

async fn post_join_group_by_code(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<JoinGroupCode>,
) -> Result<(), InvitationError> {
    Ok(try_join_group_by_code(&pool, &claims.user_id, &payload.code).await?)
}
