use crate::app_errors::AppError;
use crate::models::{Claims, GroupInfo};
use crate::utils::invitations::{
    fetch_group_info_by_code, try_create_group_invitation_with_code, try_join_group_by_code,
    GroupInvitationCreate,
};
use axum::Router;
use axum::{
    extract::Json,
    routing:: post,
    Extension,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::debug;

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
) -> Result<Json<Value>, AppError> {
    let invitation =
        try_create_group_invitation_with_code(&pool, &claims.user_id, invitation).await?;

    debug!("User {} ({}) created a group invitation successfully", &claims.user_id, &claims.login);
    Ok(Json(json!({ "code": &invitation })))
}

#[derive(Serialize, Deserialize)]
struct JoinGroupCode {
    code: String,
}

async fn post_fetch_group_info_by_code(
    _claims: Claims,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<JoinGroupCode>,
) -> Result<Json<GroupInfo>, AppError> {
    let res = fetch_group_info_by_code(&pool, &payload.code).await?;

    debug!("Group's info fetched successfully");
    Ok(Json(
        res,
    ))
}

async fn post_join_group_by_code(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<JoinGroupCode>,
) -> Result<(), AppError> {
    try_join_group_by_code(&pool, &claims.user_id, &payload.code).await?;

    debug!("User {} ({}) joined a group successfully", &claims.user_id, &claims.login);
    Ok(())
}
