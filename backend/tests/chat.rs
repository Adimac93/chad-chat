﻿use backend::utils::chat::{get_user_login_by_id, errors::ChatError, create_message};
use sqlx::PgPool;
use uuid::Uuid;

mod chat {
    use super::*;

    #[sqlx::test(fixtures("users", "groups", "group_users"))]
    async fn get_user_login_by_id_health_check(db: PgPool) {
        let res = get_user_login_by_id (
            &db,
            &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
        ).await;

        match res {
            Ok(login) if login == "Hubert".to_string() => (),
            _ => panic!("Test result is {:?}", res),
        }
    }

    #[sqlx::test(fixtures("users", "groups", "group_users"))]
    async fn get_user_login_by_id_user_does_not_exist(db: PgPool) {
        let res = get_user_login_by_id (
            &db,
            &Uuid::parse_str("a1fd5c51-326f-476e-a4f7-2e61a692bb56").unwrap(),
        ).await;

        match res {
            Err(ChatError::Unexpected(_)) => (),
            _ => panic!("Test result is {:?}", res),
        }
    }

    #[sqlx::test(fixtures("users", "groups", "group_users"))]
    async fn create_message_health_check (db: PgPool) {
        let res = create_message (
            &db,
            &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
            &Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap(),
            "Good luck then..."
        ).await;

        match res {
            Ok(_) => (),
            _ => panic!("Test result is {:?}", res),
        }
    }

    #[sqlx::test(fixtures("users", "groups", "group_users"))]
    async fn create_message_content_is_empty (db: PgPool) {
        let res = create_message (
            &db,
            &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
            &Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap(),
            "   "
        ).await;

        match res {
            Err(ChatError::EmptyMessage) => (),
            _ => panic!("Test result is {:?}", res),
        }
    }
}