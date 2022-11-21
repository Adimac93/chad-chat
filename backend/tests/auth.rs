use reqwest::StatusCode;
use backend::models::LoginCredentials;
use serde_json::json;
use uuid::Uuid;
mod tools;

mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    pub async fn user_register_test_positive(db: PgPool) {
        let addr = tools::spawn_app(db).await;
        let client = tools::client();

        let payload = json!({
            "login": format!("User{}", Uuid::new_v4()),
            "password": format!("#very#_#strong#_#pass#"),
        });

        let res = client
            .post(format!("http://{}/auth/register", addr))
            .json(&payload)
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    // todo: error result causes
    #[sqlx::test(fixtures("user"))]
    pub async fn user_register_test_negative(db: PgPool) {
        let addr = tools::spawn_app(db).await;
        let client = tools::client();

        let test_data: Vec<LoginCredentials> = vec!(
            // missing credential
            LoginCredentials::new("", "#very#_#strong#_#pass#"),
            LoginCredentials::new("   ", "#very#_#strong#_#pass#"),
            LoginCredentials::new(&format!("User{}", Uuid::new_v4()), "  "),
            LoginCredentials::new("  ", "   "),
            // weak pass
            LoginCredentials::new(&format!("User{}", Uuid::new_v4()), "12345678"),
            // user already exists
            LoginCredentials::new("some_user", "#very#_#strong#_#pass#"),
            LoginCredentials::new("some_user", "#strong#_#pass#"),
        );

        for elem in test_data {
            let payload = json!({
                "login": elem.login,
                "password": elem.password,
            });
    
            let res = client
                .post(format!("http://{}/auth/register", addr))
                .json(&payload)
                .send()
                .await
                .unwrap();
    
            assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        }
    }

    #[sqlx::test]
    pub async fn auth_integration_test(db: PgPool) {
        let addr = tools::spawn_app(db).await;
        let client = tools::client();

        let payload = json!({
            "login": format!("User{}", Uuid::new_v4()),
            "password": format!("#very#_#strong#_#pass#"),
        });

        let res = client
            .post(format!("http://{}/auth/register", addr))
            .json(&payload)
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        let res = client
            .post(format!("http://{}/auth/login", addr))
            .json(&payload)
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        let res = client
            .post(format!("http://{}/auth/user-validation", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }
}
