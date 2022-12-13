use reqwest::StatusCode;
mod tools;

mod service {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn health_check(db: PgPool) {
        let app_data = tools::AppData::new(db).await;

        let res = app_data
            .client()
            .get(format!("http://{}/api/health", app_data.addr))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK)
    }

    #[sqlx::test]
    async fn not_found(db: PgPool) {
        let app_data = tools::AppData::new(db).await;

        let res = app_data
            .client()
            .get(format!(
                "http://{}/{}",
                app_data.addr, "this_side_should_never_exist"
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::NOT_FOUND)
    }
}
