use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use dotenv::dotenv;
mod tools;

#[derive(Deserialize)]
struct TokenData {
    access_token: String,
}

mod tests {
    use super::*;

    #[tokio::test]
    pub async fn auth_test() {
        dotenv().ok();

        let addr = tools::spawn_app().await;
        let client = reqwest::Client::new();

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

        let token = res.json::<TokenData>().await.unwrap().access_token;

        let res = client
            .post(format!("http://{}/auth/user-validation", addr))
            .bearer_auth(token)
            .send()
            .await
            .unwrap();
        
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    pub async fn bad_password_test() {
        dotenv().ok();

        let addr = tools::spawn_app().await;
        let client = reqwest::Client::new();

        let user_id = Uuid::new_v4();

        let payload = json!({
            "login": format!("User{}", user_id.clone()),
            "password": format!("#very#_#strong#_#pass#"),
        });

        let bad_payload = json!({
            "login": format!("User{}", user_id.clone()),
            "password": format!("#very#_#bad#_#pass#"),
        });

        client
            .post(format!("http://{}/auth/register", addr))
            .json(&payload)
            .send()
            .await
            .unwrap();

        let res = client
            .post(format!("http://{}/auth/login", addr))
            .json(&bad_payload)
            .send()
            .await
            .unwrap();
        
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    pub async fn bad_login_test() {
        dotenv().ok();

        let addr = tools::spawn_app().await;
        let client = reqwest::Client::new();

        let user_id = Uuid::new_v4();

        let payload = json!({
            "login": format!("User{}", user_id.clone()),
            "password": format!("#very#_#strong#_#pass#"),
        });

        let bad_payload = json!({
            "login": format!("A_User{}", user_id.clone()),
            "password": format!("#very#_#strong#_#pass#"),
        });

        client
            .post(format!("http://{}/auth/register", addr))
            .json(&payload)
            .send()
            .await
            .unwrap();

        let res = client
            .post(format!("http://{}/auth/login", addr))
            .json(&bad_payload)
            .send()
            .await
            .unwrap();
        
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
