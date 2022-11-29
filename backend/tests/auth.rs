use backend::models::LoginCredentials;
use backend::utils::auth::{errors::AuthError, login_user, try_register_user};
use nanoid::nanoid;
use reqwest::StatusCode;
use secrecy::SecretString;
use serde_json::json;
mod tools;

mod auth {
    use super::*;
    use backend::utils::auth::{errors::AuthError, login_user, try_register_user};
    use reqwest::Response;
    use secrecy::SecretString;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn registration_health_check(db: PgPool) {
        let res = try_register_user(
            &db,
            &format!("User{}", nanoid!(10)),
            SecretString::new("#very#_#strong#_#pass#".to_string()),
            "Chad",
        )
        .await;

        match res {
            Ok(_) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_missing_credential_0(db: PgPool) {
        let res = try_register_user(
            &db,
            "",
            SecretString::new("#very#_#strong#_#pass#".to_string()),
            "Chad",
        )
        .await;

        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_missing_credential_1(db: PgPool) {
        let res = try_register_user(
            &db,
            "   ",
            SecretString::new("#very#_#strong#_#pass#".to_string()),
            "Chad",
        )
        .await;

        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_missing_credential_2(db: PgPool) {
        let res = try_register_user(
            &db,
            &format!("User{}", nanoid!(10)),
            SecretString::new("  ".to_string()),
            "Chad",
        )
        .await;

        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_missing_credential_3(db: PgPool) {
        let res = try_register_user(&db, "  ", SecretString::new("   ".to_string()), "Chad").await;

        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_weak_password(db: PgPool) {
        let res = try_register_user(
            &db,
            &format!("User{}", nanoid!(10)),
            SecretString::new("12345678".to_string()),
            "Chad",
        )
        .await;

        match res {
            Err(AuthError::WeakPassword) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_user_exists_0(db: PgPool) {
        let res = try_register_user(
            &db,
            "some_user",
            SecretString::new("#very#_#strong#_#pass#".to_string()),
            "Chad",
        )
        .await;

        match res {
            Err(AuthError::UserAlreadyExists) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_user_exists_1(db: PgPool) {
        let res = try_register_user(
            &db,
            "some_user",
            SecretString::new("#strong#_#pass#".to_string()),
            "Chad",
        )
        .await;

        match res {
            Err(AuthError::UserAlreadyExists) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_invalid_username_0(db: PgPool) {
        let res = try_register_user(
            &db,
            "why",
            SecretString::new("#strong#_#pass#".to_string()),
            "Chad",
        )
        .await;

        match res {
            Err(AuthError::InvalidUsername(_)) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_invalid_username_1(db: PgPool) {
        let res = try_register_user(
            &db,
            "spaced name",
            SecretString::new("#strong#_#pass#".to_string()),
            "Chad",
        )
        .await;

        match res {
            Err(AuthError::InvalidUsername(_)) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_invalid_username_2(db: PgPool) {
        let res = try_register_user(
            &db,
            "verylongveryverylongnameveryveryverylongname",
            SecretString::new("#strong#_#pass#".to_string()),
            "Chad",
        )
        .await;

        match res {
            Err(AuthError::InvalidUsername(_)) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_health_check(db: PgPool) {
        let res = login_user(
            &db,
            "some_user",
            SecretString::new("#strong#_#pass#".to_string()),
        )
        .await;

        match res {
            Ok(_) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_missing_credential_0(db: PgPool) {
        let res = login_user(&db, "some_user", SecretString::new("   ".to_string())).await;

        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_missing_credential_1(db: PgPool) {
        let res = login_user(
            &db,
            "    ",
            SecretString::new("#strong#_#pass#".to_string()),
        )
        .await;

        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_missing_credential_2(db: PgPool) {
        let res = login_user(&db, "    ", SecretString::new("  ".to_string())).await;

        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_no_user_found(db: PgPool) {
        let res = login_user(
            &db,
            "different_user",
            SecretString::new("#strong#_#pass#".to_string()),
        )
        .await;

        match res {
            Err(AuthError::WrongUserOrPassword) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_wrong_password(db: PgPool) {
        let res = login_user(
            &db,
            "some_user",
            SecretString::new("#wrong#_#pass#".to_string()),
        )
        .await;

        match res {
            Err(AuthError::WrongUserOrPassword) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test]
    async fn auth_integration_test(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let client = app_data.client();

        let payload = json!({
            "login": format!("User{}", nanoid!(10)),
            "password": format!("#very#_#strong#_#pass#"),
            "nickname": format!("Chad")
        });

        let res = app_data
            .client()
            .post(format!("http://{}/auth/register", app_data.addr))
            .json(&payload)
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        let res = client
            .post(format!("http://{}/auth/login", app_data.addr))
            .json(&payload)
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        let res = client
            .post(format!("http://{}/auth/user-validation", app_data.addr))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }
}
