use reqwest::StatusCode;
mod tools;

use sqlx::PgPool;

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
async fn method_not_allowed(db: PgPool) {
    let app_data = tools::AppData::new(db).await;

    let res = app_data
        .client()
        .post(format!("http://{}/api", app_data.addr))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED)
}
