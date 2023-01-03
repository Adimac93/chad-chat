mod tools;

use backend::utils::friends::{
    fetch_user_friends, remove_user_friend, respond_to_friend_request,
    send_friend_request_by_user_id, update_friend_note,
};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(fixtures("users", "credentials"))]
async fn send_request(db: PgPool) {
    let user_id = Uuid::parse_str("ba34ff10-4b89-44cb-9b36-31eb57c41556").unwrap(); // Adam
    let request_user_id = Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(); // Hubert
    let res = send_friend_request_by_user_id(&db, user_id, request_user_id).await;

    res.unwrap();
}

#[sqlx::test(fixtures("users", "credentials", "friend_requests"))]
pub async fn accept_request(db: PgPool) {
    let sender_id = Uuid::parse_str("ba34ff10-4b89-44cb-9b36-31eb57c41556").unwrap();
    let receiver_id = Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap();
    respond_to_friend_request(&db, true, sender_id, receiver_id)
        .await
        .unwrap();
}

pub async fn decline_request(db: PgPool) {
    let sender_id = Uuid::parse_str("ba34ff10-4b89-44cb-9b36-31eb57c41556").unwrap();
    let receiver_id = Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap();
    respond_to_friend_request(&db, false, sender_id, receiver_id)
        .await
        .unwrap();
}

#[sqlx::test(fixtures("users", "credentials", "friends"))]
pub async fn fetch_all_friends(db: PgPool) {
    let user_id = Uuid::parse_str("4bd30a6a-7dfe-46a2-b741-f49612aa85c1").unwrap();
    let friends = fetch_user_friends(&db, user_id).await.unwrap();
    assert_eq!(friends.len(), 1)
}

#[sqlx::test(fixtures("users", "credentials", "friends"))]
pub async fn remove_friend(db: PgPool) {
    let user_id = Uuid::parse_str("4bd30a6a-7dfe-46a2-b741-f49612aa85c1").unwrap();
    let friend_id = Uuid::parse_str("6666e44f-14ce-4aa5-b5f9-8a4cc5ee5c58").unwrap();
    remove_user_friend(&db, user_id, friend_id).await.unwrap();
}

#[sqlx::test(fixtures("users", "credentials", "friends"))]
pub async fn update_note(db: PgPool) {
    let user_id = Uuid::parse_str("4bd30a6a-7dfe-46a2-b741-f49612aa85c1").unwrap();
    let friend_id = Uuid::parse_str("6666e44f-14ce-4aa5-b5f9-8a4cc5ee5c58").unwrap();
    let note = "Polo is no longer my best friend!".into();
    update_friend_note(&db, user_id, friend_id, note)
        .await
        .unwrap();
}
