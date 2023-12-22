use axum::{routing::{put, get}, extract::{State, Path}, Router, debug_handler, Json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{state::{RdPool, AppState}, errors::AppError, utils::{roles::{models::{ReceiveRoleOutput, PrivilegeChangeInput, UserRoleChangeInput}, set_role, get_user_role, set_privileges}, auth::models::Claims}};


pub fn router() -> Router<AppState> {
    Router::new()
        .route("/privileges", put(change_privileges))
        .route("/:user_id", put(set_user_role))
        .route("/:user_id", get(receive_user_role))
}

#[debug_handler(state = AppState)]
async fn set_user_role(
    claims: Claims,
    State(pg): State<PgPool>,
    State(mut rd): State<RdPool>,
    Path(target_user_id): Path<Uuid>,
    Json(data): Json<UserRoleChangeInput>,
) -> Result<(), AppError> {
    set_role(&pg, &mut rd, claims.user_id, target_user_id, &data).await?;
    Ok(())
}

#[debug_handler(state = AppState)]
async fn receive_user_role(
    State(pg): State<PgPool>,
    State(mut rd): State<RdPool>,
    Path((group_id, target_user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ReceiveRoleOutput>, AppError> {
    let res = get_user_role(&pg, &mut rd, target_user_id, group_id).await?;

    Ok(Json(ReceiveRoleOutput { role: res }))
}

#[debug_handler(state = AppState)]
async fn change_privileges(
    claims: Claims,
    State(pg): State<PgPool>,
    State(mut rd): State<RdPool>,
    Json(data): Json<PrivilegeChangeInput>,
) -> Result<(), AppError> {
    set_privileges(&pg, &mut rd, claims.user_id, &data).await?;
    Ok(())
}
