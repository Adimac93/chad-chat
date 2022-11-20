use reqwest::StatusCode;
use backend::models::LoginCredentials;
use serde_json::json;
use uuid::Uuid;
mod tools;

mod tests {
    use super::*;

    #[tokio::test]
    pub async fn user_register_test_positive() {
        let addr = tools::spawn_app().await;
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
    #[tokio::test]
    pub async fn user_register_test_negative() {
        let addr = tools::spawn_app().await;
        let client = tools::client();

        let repeating_user = format!("User{}", Uuid::new_v4());

        let payload = json!({
            "login": repeating_user,
            "password": "#very#_#strong#_#pass#",
        });

        let _res = client
            .post(format!("http://{}/auth/register", addr))
            .json(&payload)
            .send()
            .await
            .unwrap();

        let test_data: Vec<LoginCredentials> = vec!(
            // missing credential
            LoginCredentials::new("", "#very#_#strong#_#pass#"),
            LoginCredentials::new("   ", "#very#_#strong#_#pass#"),
            LoginCredentials::new(&format!("User{}", Uuid::new_v4()), "  "),
            LoginCredentials::new("  ", "   "),
            // weak pass
            LoginCredentials::new(&format!("User{}", Uuid::new_v4()), "12345678"),
            // user already exists
            LoginCredentials::new(&repeating_user, "#very#_#strong#_#pass#"),
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

    #[tokio::test]
    pub async fn auth_integration_test() {
        let addr = tools::spawn_app().await;
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
