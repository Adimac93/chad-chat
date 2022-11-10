use std::collections::HashMap;
use std::sync::Arc;

use crate::groups::{GroupError, try_add_user_to_group};
use crate::models::{Claims, GroupUser, NewGroup};
use crate::queries::{check_if_group_member, create_group, AppError};
use axum::extract::Path;
use axum::{debug_handler, extract, Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;

pub async fn post_create_group(
    claims: Claims,
    pool: Extension<PgPool>,
    group: extract::Json<NewGroup>,
) -> Result<(), AppError> {
    tracing::trace!("JWT: {:#?}", claims);
    create_group(&pool, group.name.trim(), claims.id).await
}

pub async fn post_add_user_to_group(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    Json(GroupUser { user_id, group_id }): Json<GroupUser>,
) -> Result<(), GroupError> {
    tracing::trace!("JWT: {:#?}", claims);
    try_add_user_to_group(&pool, &user_id, &group_id).await?;
    Ok(())
}

pub struct InvitationState {
    code: RwLock<HashMap<Uuid, Uuid>>,
    // invitation : group
}

impl InvitationState {
    pub fn new() -> Self {
        InvitationState {
            code: RwLock::new(HashMap::new()),
        }
    }
}

#[derive(Deserialize)]
pub struct NewGroupInvitation {
    group_id: Uuid,
}

pub async fn post_create_group_invitation_link(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    Json(NewGroupInvitation { group_id }): Json<NewGroupInvitation>,
    state: Extension<Arc<InvitationState>>,
) -> Result<Json<Value>, GroupError> {
    let is_member = check_if_group_member(&pool, &claims.id, &group_id)
        .await
        .map_err(|e| GroupError::Unexpected(e.into()))?;
    let id = Uuid::new_v4();
    if is_member {
        let _ = state.code.write().await.insert(id, group_id);
    }

    Ok(Json(json!({
        "url": format!("Your invitation link: 127.0.0.1:3000/groups/join/{id}")
    })))
}

pub async fn get_join_group_by_link(
    Path(invite_id): Path<Uuid>,
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    state: Extension<Arc<InvitationState>>,
) -> Result<(), GroupError> {
    match state.code.read().await.get(&invite_id) {
        Some(group_id) => {
            try_add_user_to_group(&pool, &claims.id, group_id)
                .await?;
            Ok(())
        }
        None => Err(GroupError::BadInvitation),
    }
}
