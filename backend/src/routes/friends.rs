use axum::{extract::State, Json};
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

use crate::state::AppState;
use crate::utils::friends::models::{
    FriendInvitationResponse, FriendList, IdentifiedFriendIvitation,
};
use crate::utils::friends::{
    fetch_friends, respond_to_friend_request, send_friend_request_by_user_id,
    send_friend_request_by_username_and_tag, TaggedUsername,
};
use crate::{errors::AppError, utils::auth::models::Claims};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(friends))
        .route("/invitations/id", post(send_id))
        .route("/invitations/tag", post(send_tag))
        .route("/invitations/respond", post(respond))
}

async fn friends(claims: Claims, State(pool): State<PgPool>) -> Result<Json<FriendList>, AppError> {
    let friends = fetch_friends(&pool, claims.user_id).await?;
    Ok(Json(FriendList { friends }))
}

async fn send_tag(
    claims: Claims,
    State(pool): State<PgPool>,
    Json(data): Json<TaggedUsername>,
) -> Result<(), AppError> {
    send_friend_request_by_username_and_tag(&pool, claims.user_id, data).await?;
    Ok(())
}

async fn send_id(
    claims: Claims,
    State(pool): State<PgPool>,
    Json(data): Json<IdentifiedFriendIvitation>,
) -> Result<(), AppError> {
    send_friend_request_by_user_id(&pool, claims.user_id, data.user_id).await?;
    Ok(())
}

async fn respond(
    claims: Claims,
    State(pool): State<PgPool>,
    Json(data): Json<FriendInvitationResponse>,
) -> Result<(), AppError> {
    respond_to_friend_request(&pool, data.sender_id, claims.user_id, data.is_accepted).await?;
    Ok(())
}
