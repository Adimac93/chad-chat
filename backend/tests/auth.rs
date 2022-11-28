use backend::models::LoginCredentials;
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;
mod tools;

mod auth {
    use backend::utils::auth::{try_register_user, errors::AuthError, login_user};
    use reqwest::Response;
    use secrecy::SecretString;
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn user_register_test_positive(db: PgPool) {
        let res = try_register_user(
            &db,
            &format!("User{}", Uuid::new_v4()),
            SecretString::new("#very#_#strong#_#pass#".to_string())
        ).await;

        match res {
            Ok(_) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_returns_400_if_missing_credential_0(db: PgPool) {
        let res = try_register_user(
            &db,
            "",
            SecretString::new("#very#_#strong#_#pass#".to_string())
        ).await;
        
        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_returns_400_if_missing_credential_1(db: PgPool) {
        let res = try_register_user(
            &db,
            "   ",
            SecretString::new("#very#_#strong#_#pass#".to_string())
        ).await;
        
        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_returns_400_if_missing_credential_2(db: PgPool) {
        let res = try_register_user(
            &db,
            &format!("User{}", Uuid::new_v4()),
            SecretString::new("  ".to_string())
        ).await;
        
        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_returns_400_if_missing_credential_3(db: PgPool) {
        let res = try_register_user(
            &db,
            "  ",
            SecretString::new("   ".to_string())
        ).await;
        
        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_returns_400_if_weak_password(db: PgPool) {
        let res = try_register_user(
            &db,
            &format!("User{}", Uuid::new_v4()),
            SecretString::new("12345678".to_string())
        ).await;
        
        match res {
            Err(AuthError::WeakPassword) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_returns_400_if_user_exists_0(db: PgPool) {
        let res = try_register_user(
            &db,
            "some_user",
            SecretString::new("#very#_#strong#_#pass#".to_string())
        ).await;
        
        match res {
            Err(AuthError::UserAlreadyExists) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_returns_400_if_user_exists_1(db: PgPool) {
        let res = try_register_user(
            &db,
            "some_user",
            SecretString::new("#strong#_#pass#".to_string())
        ).await;
        
        match res {
            Err(AuthError::UserAlreadyExists) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_returns_200_if_valid_credentials(db: PgPool) {
        let res = login_user(
            &db,
            "some_user",
            SecretString::new("#strong#_#pass#".to_string())
        ).await;
        
        match res {
            Ok(_) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_returns_400_if_missing_credential_0(db: PgPool) {
        let res = login_user(
            &db,
            "some_user",
            SecretString::new("   ".to_string())
        ).await;
        
        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_returns_400_if_missing_credential_1(db: PgPool) {
        let res = login_user(
            &db,
            "    ",
            SecretString::new("#strong#_#pass#".to_string())
        ).await;
        
        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_returns_400_if_missing_credential_2(db: PgPool) {
        let res = login_user(
            &db,
            "    ",
            SecretString::new("  ".to_string())
        ).await;
        
        match res {
            Err(AuthError::MissingCredential) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_returns_401_if_no_user_found(db: PgPool) {
        let res = login_user(
            &db,
            "different_user",
            SecretString::new("#strong#_#pass#".to_string())
        ).await;
        
        match res {
            Err(AuthError::WrongUserOrPassword) => (),
            _ => panic!("Test gives the result {:?}", res),
        }
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_returns_401_if_wrong_password(db: PgPool) {
        let res = login_user(
            &db,
            "some_user",
            SecretString::new("#wrong#_#pass#".to_string())
        ).await;
        
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
            "login": format!("User{}", Uuid::new_v4()),
            "password": format!("#very#_#strong#_#pass#"),
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
