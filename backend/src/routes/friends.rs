use axum::{
    routing::{get, post},
    Router,
};
use axum::{Extension, Json};
use sqlx::PgPool;

use crate::utils::friends::models::{
    FriendInvitationResponse, FriendList, IdentifiedFriendIvitation,
};
use crate::utils::friends::{
    fetch_user_friends, respond_to_friend_request, send_friend_request_by_user_id,
};
use crate::{app_errors::AppError, utils::auth::models::Claims};

pub fn router() -> Router {
    Router::new()
        .route("/", get(user_friends))
        .route("/invitations/id", post(send_friend_invitation_by_id))
        //.route("/invitations/tag", get(user_invitations))
        .route("/invitations/respond", post(respond_to_invitation))
}

async fn user_friends(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<FriendList>, AppError> {
    let friends = fetch_user_friends(&pool, claims.user_id).await?;
    Ok(Json(FriendList { friends }))
}

async fn send_friend_invitation_by_id(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    Json(data): Json<IdentifiedFriendIvitation>,
) -> Result<(), AppError> {
    send_friend_request_by_user_id(&pool, claims.user_id, data.user_id).await?;
    Ok(())
}

async fn respond_to_invitation(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
    Json(data): Json<FriendInvitationResponse>,
) -> Result<(), AppError> {
    respond_to_friend_request(&pool, data.is_accepted, data.sender_id, claims.user_id).await?;
    Ok(())
}

// async fn send_friend_invitation_by_tag(
//     claims: Claims,
//     Extension(pool): Extension<PgPool>,
//     Json(data): Json<TaggedFriendInvitation>,
// ) -> Result<(), AppError> {
//     let res = send_friend_request_by_user_tag(&pool, claims.user_id, data.user_id).await?;
// }
