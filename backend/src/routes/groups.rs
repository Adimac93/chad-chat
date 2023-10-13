use crate::errors::AppError;
use crate::state::AppState;
use crate::utils::auth::models::Claims;
use crate::utils::groups::models::NewGroup;
use crate::utils::groups::*;
use axum::extract::State;
use axum::Router;
use axum::{extract::Json, routing::get};
use serde_json::Value;
use sqlx::PgPool;
use tracing::debug;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_user_groups).post(post_create_group))
    // .route("/leave", post(leave_group))
}

async fn get_user_groups(
    claims: Claims,
    State(pool): State<PgPool>,
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
    State(pool): State<PgPool>,
    group: Json<NewGroup>,
) -> Result<(), AppError> {
    tracing::trace!("JWT: {:#?}", claims);
    let res = create_group(&pool, group.name.trim(), claims.user_id).await?;

    debug!("Group {} created successfully", group.name);

    Ok(res)
}

// async fn leave_group(
//     claims: Claims,
//     State(pool): State<PgPool>,
//     Json(group_id): Json<Uuid>,
//  ) -> Result<(), AppError> {
//     tracing::trace!("JWT: {:#?}", claims);
//     Ok(try_remove_user_from_group(&pool, claims.user_id, group_id).await?)
// }
