use crate::models::{Claims, GroupUser, InvitationState, NewGroup, NewGroupInvitation};
use crate::utils::groups::errors::*;
use crate::utils::groups::*;
use axum::Router;
use axum::{
    extract::{Json, Path},
    routing::{get, post},
    Extension,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub fn router() -> Router {
    Router::new()
        .route("/", get(get_user_groups).post(post_create_group))
        .route("/add-user", post(post_add_user_to_group))
        .route("/invite", post(post_create_group_invitation_link))
        .route("/join/:invite_id", get(get_join_group_by_link))
        .route("/info/:invite_id", get(get_invitation_info))
        .layer(Extension(Arc::new(InvitationState::new())))
}

async fn get_user_groups(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Value>, GroupError> {
    query_user_groups(&pool, claims.id).await
}

async fn post_create_group(
    claims: Claims,
    pool: Extension<PgPool>,
    group: Json<NewGroup>,
) -> Result<(), GroupError> {
    tracing::trace!("JWT: {:#?}", claims);
    create_group(&pool, group.name.trim(), claims.id).await
}

async fn post_add_user_to_group(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    Json(GroupUser { user_id, group_id }): Json<GroupUser>,
) -> Result<(), GroupError> {
    tracing::trace!("JWT: {:#?}", claims);
    try_add_user_to_group(&pool, &user_id, &group_id).await?;
    Ok(())
}

async fn post_create_group_invitation_link(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    Json(NewGroupInvitation { group_id }): Json<NewGroupInvitation>,
    state: Extension<Arc<InvitationState>>,
) -> Result<Json<Value>, GroupError> {
    match check_if_group_member(&pool, &claims.id, &group_id).await? {
        true => {
            let id = Uuid::new_v4();
            let _ = state.code.write().await.insert(id, group_id);
            Ok(Json(json!({ "id": id })))
        }
        false => Err(GroupError::UserNotInGroup),
    }
}

pub async fn get_join_group_by_link(
    Path(invite_id): Path<Uuid>,
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    state: Extension<Arc<InvitationState>>,
) -> Result<(), GroupError> {
    match state.code.read().await.get(&invite_id) {
        Some(group_id) => {
            try_add_user_to_group(&pool, &claims.id, group_id).await?;
            Ok(())
        }
        None => Err(GroupError::BadInvitation),
    }
}

pub async fn get_invitation_info(
    Path(invite_id): Path<Uuid>,
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    state: Extension<Arc<InvitationState>>,
) -> Result<Json<GroupInfo>, GroupError> {
    match state.code.read().await.get(&invite_id) {
        Some(group_id) => {
            let info = get_group_info(&pool, group_id).await?;
            Ok(Json(info))
        }
        None => Err(GroupError::BadInvitation),
    }
}
