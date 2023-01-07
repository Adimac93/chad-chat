use crate::utils::roles::*;
use crate::utils::roles::errors::RoleError;
use axum::Router;
use axum::routing::post;
use axum::{extract::Json, Extension};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;


// endpoints for test purposes
pub fn router() -> Router {
    Router::new()
        .route("/privileges", post(post_set_privileges))
        .route("/users", post(post_set_roles))
        .route("/get", post(get_roles))
}

pub async fn post_set_privileges(
    Extension(pool): Extension<PgPool>,
    Json(group_id): Json<Uuid>,
    Json(privileges): Json<GroupRolePrivileges>,
) -> Result<Json<Value>, RoleError> {
    let privileges = set_group_role_privileges(&pool, group_id, privileges).await?;
    Ok(Json(json!({ "group_privileges": privileges })))
}

pub async fn post_set_roles(Extension(pool): Extension<PgPool>, Json(roles): Json<GroupUsersRoleFromJson>) -> Result<(), RoleError> {
    set_group_users_role(&pool, GroupUsersRole::try_from(roles)?).await
}

pub async fn get_roles(Extension(pool): Extension<PgPool>, Json(group_id): Json<Uuid>) -> Result<Json<Value>, RoleError> {
    let privileges = get_group_role_privileges(&pool, group_id).await?;
    Ok(Json(json!({ "group_privileges": privileges })))
}