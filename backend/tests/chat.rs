use backend::utils::chat::{create_message, get_user_email_by_id};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(fixtures("users", "credentials", "groups", "roles", "group_users"))]
async fn get_user_email_by_id_health_check(db: PgPool) {
    let res = get_user_email_by_id(
        &db,
        &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
    )
    .await;

    match res {
        Ok(email) if email == *"Hubert@gmail.com" => (),
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "credentials", "groups", "roles", "group_users"))]
async fn get_user_email_by_id_user_does_not_exist(db: PgPool) {
    let res = get_user_email_by_id(
        &db,
        &Uuid::parse_str("a1fd5c51-326f-476e-a4f7-2e61a692bb56").unwrap(),
    )
    .await;

    assert!(res.is_err())
}

#[sqlx::test(fixtures("users", "credentials", "groups", "roles", "group_users"))]
async fn create_message_health_check(db: PgPool) {
    let res = create_message(
        &db,
        &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
        &Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap(),
        "Good luck then...",
    )
    .await;

    match res {
        Ok(_) => (),
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "credentials", "groups", "roles", "group_users"))]
async fn create_message_content_is_empty(db: PgPool) {
    let res = create_message(
        &db,
        &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
        &Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap(),
        "   ",
    )
    .await;

    assert!(res.is_err())
}
