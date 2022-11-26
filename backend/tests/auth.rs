use backend::models::LoginCredentials;
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;
mod tools;

mod tests {
    use reqwest::Response;
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn user_register_test_positive(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials =
            LoginCredentials::new(&format!("User{}", Uuid::new_v4()), "#very#_#strong#_#pass#");

        let res = send_test_request(app_data, credentials, "/auth/register").await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[sqlx::test(fixtures("user"))]
    async fn registration_returns_400_if_missing_credential_0(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new("", "#very#_#strong#_#pass#");

        let res = send_test_request(app_data, credentials, "/auth/register").await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(fixtures("user"))]
    async fn registation_returns_400_if_missing_credential_1(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new("   ", "#very#_#strong#_#pass#");

        let res = send_test_request(app_data, credentials, "/auth/register").await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(fixtures("user"))]
    async fn registation_returns_400_if_missing_credential_2(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new(&format!("User{}", Uuid::new_v4()), "  ");

        let res = send_test_request(app_data, credentials, "/auth/register").await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(fixtures("user"))]
    async fn registation_returns_400_if_missing_credential_3(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new("  ", "   ");

        let res = send_test_request(app_data, credentials, "/auth/register").await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(fixtures("user"))]
    async fn registation_returns_400_if_weak_password(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new(&format!("User{}", Uuid::new_v4()), "12345678");

        let res = send_test_request(app_data, credentials, "/auth/register").await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(fixtures("user"))]
    async fn registation_returns_400_if_user_exists_0(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new("some_user", "#very#_#strong#_#pass#");

        let res = send_test_request(app_data, credentials, "/auth/register").await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(fixtures("user"))]
    async fn registation_returns_400_if_user_exists_1(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new("some_user", "#strong#_#pass#");

        let res = send_test_request(app_data, credentials, "/auth/register").await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    async fn send_test_request(
        app_data: tools::AppData,
        credentials: LoginCredentials,
        path: &str,
    ) -> Response {
        let payload = json!({
            "login": credentials.login,
            "password": credentials.password,
        });

        app_data
            .client()
            .post(format!("http://{}{}", app_data.addr, path))
            .json(&payload)
            .send()
            .await
            .unwrap()
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_returns_200_if_valid_credentials(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new("some_user", "#strong#_#pass#");

        let res = send_test_request(app_data, credentials, "/auth/login").await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_returns_400_if_missing_credential_0(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new("some_user", "   ");

        let res = send_test_request(app_data, credentials, "/auth/login").await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_returns_400_if_missing_credential_1(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new("    ", "#strong#_#pass#");

        let res = send_test_request(app_data, credentials, "/auth/login").await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_returns_400_if_missing_credential_2(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new("    ", "  ");

        let res = send_test_request(app_data, credentials, "/auth/login").await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_returns_401_if_no_user_found(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new("different_user", "#strong#_#pass#");

        let res = send_test_request(app_data, credentials, "/auth/login").await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test(fixtures("user"))]
    async fn login_returns_401_if_wrong_password(db: PgPool) {
        let app_data = tools::AppData::new(db).await;
        let credentials = LoginCredentials::new("some_user", "#wrong#_#pass#");

        let res = send_test_request(app_data, credentials, "/auth/login").await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
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
