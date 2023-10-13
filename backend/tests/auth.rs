use nanoid::nanoid;
use reqwest::StatusCode;
use serde_json::json;
mod tools;

use crate::utils::auth::{errors::AuthError, try_register_user, verify_user_credentials};
use secrecy::SecretString;
use sqlx::PgPool;

// #[sqlx::test]
// async fn registration_health_check(db: PgPool) {
//     let res = try_register_user(
//         &db,
//         None,
//         &format!("User{}@gmail.com", nanoid!(10)),
//         SecretString::new("#very#_#strong#_#pass#".to_string()),
//         "Chad",
//     )
//     .await;

//     match res {
//         Ok(_) => (),
//         _ => panic!("Test gives the result {:?}", res),
//     }
// }

// #[sqlx::test(fixtures("users", "credentials"))]
// async fn registration_missing_credential_0(db: PgPool) {
//     let res = try_register_user(
//         &db,
//         None,
//         "",
//         SecretString::new("#very#_#strong#_#pass#".to_string()),
//         "Chad",
//     )
//     .await;

//     match res {
//         Err(AuthError::MissingCredential) => (),
//         _ => panic!("Test gives the result {:?}", res),
//     }
// }

// #[sqlx::test(fixtures("users", "credentials"))]
// async fn registration_missing_credential_1(db: PgPool) {
//     let res = try_register_user(
//         &db,
//         None,
//         "   ",
//         SecretString::new("#very#_#strong#_#pass#".to_string()),
//         "Chad",
//     )
//     .await;

//     match res {
//         Err(AuthError::MissingCredential) => (),
//         _ => panic!("Test gives the result {:?}", res),
//     }
// }

// #[sqlx::test(fixtures("users", "credentials"))]
// async fn registration_missing_credential_2(db: PgPool) {
//     let res = try_register_user(
//         &db,
//         None,
//         &format!("User{}", nanoid!(10)),
//         SecretString::new("  ".to_string()),
//         "Chad",
//     )
//     .await;

//     match res {
//         Err(AuthError::MissingCredential) => (),
//         _ => panic!("Test gives the result {:?}", res),
//     }
// }

// #[sqlx::test(fixtures("users", "credentials"))]
// async fn registration_missing_credential_3(db: PgPool) {
//     let res = try_register_user(
//         &db,
//         None,
//         "  ",
//         SecretString::new("   ".to_string()),
//         "Chad",
//     )
//     .await;

//     match res {
//         Err(AuthError::MissingCredential) => (),
//         _ => panic!("Test gives the result {:?}", res),
//     }
// }

// #[sqlx::test(fixtures("users", "credentials"))]
// async fn registration_weak_password(db: PgPool) {
//     let res = try_register_user(
//         &db,
//         None,
//         &format!("User{}@gmail.com", nanoid!(10)),
//         SecretString::new("12345678".to_string()),
//         "Chad",
//     )
//     .await;

//     match res {
//         Err(AuthError::WeakPassword) => (),
//         _ => panic!("Test gives the result {:?}", res),
//     }
// }

// #[sqlx::test(fixtures("users", "credentials"))]
// async fn registration_user_exists_0(db: PgPool) {
//     let res = try_register_user(
//         &db,
//         None,
//         "some_user@gmail.com",
//         SecretString::new("#very#_#strong#_#pass#".to_string()),
//         "Chad",
//     )
//     .await;

//     match res {
//         Err(AuthError::UserAlreadyExists) => (),
//         _ => panic!("Test gives the result {:?}", res),
//     }
// }

// #[sqlx::test(fixtures("users", "credentials"))]
// async fn registration_user_exists_1(db: PgPool) {
//     let res = try_register_user(
//         &db,
//         None,
//         "some_user@gmail.com",
//         SecretString::new("#strong#_#pass#".to_string()),
//         "Chad",
//     )
//     .await;

//     match res {
//         Err(AuthError::UserAlreadyExists) => (),
//         _ => panic!("Test gives the result {:?}", res),
//     }
// }

// #[sqlx::test(fixtures("users", "credentials"))]
// async fn registration_invalid_username_0(db: PgPool) {
//     let res = try_register_user(
//         &db,
//         None,
//         "why",
//         SecretString::new("#strong#_#pass#".to_string()),
//         "Chad",
//     )
//     .await;

//     match res {
//         Err(AuthError::InvalidEmail(_)) => (),
//         _ => panic!("Test gives the result {:?}", res),
//     }
// }

// #[sqlx::test(fixtures("users", "credentials"))]
// async fn registration_invalid_username_1(db: PgPool) {
//     let res = try_register_user(
//         &db,
//         None,
//         "spaced name@gmail.com",
//         SecretString::new("#strong#_#pass#".to_string()),
//         "Chad",
//     )
//     .await;

//     match res {
//         Err(AuthError::InvalidEmail(_)) => (),
//         _ => panic!("Test gives the result {:?}", res),
//     }
// }

// #[sqlx::test(fixtures("users", "credentials"))]
// async fn registration_invalid_username_2(db: PgPool) {
//     let res = try_register_user(
//         &db,
//         None,
//         "verylongveryverylongnameveryveryverylongname",
//         SecretString::new("#strong#_#pass#".to_string()),
//         "Chad",
//     )
//     .await;

//     match res {
//         Err(AuthError::InvalidEmail(_)) => (),
//         _ => panic!("Test gives the result {:?}", res),
//     }
// }

#[sqlx::test(fixtures("users", "credentials"))]
async fn login_health_check(db: PgPool) {
    let res = verify_user_credentials(
        &db,
        "some_user@gmail.com",
        SecretString::new("#strong#_#pass#".to_string()),
    )
    .await;

    match res {
        Ok(_) => (),
        _ => panic!("Test gives the result {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "credentials"))]
async fn login_missing_credential_0(db: PgPool) {
    let res = verify_user_credentials(&db, "some_user", SecretString::new("   ".to_string())).await;

    match res {
        Err(AuthError::MissingCredential) => (),
        _ => panic!("Test gives the result {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "credentials"))]
async fn login_missing_credential_1(db: PgPool) {
    let res = verify_user_credentials(
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

#[sqlx::test(fixtures("users", "credentials"))]
async fn login_missing_credential_2(db: PgPool) {
    let res = verify_user_credentials(&db, "    ", SecretString::new("  ".to_string())).await;

    match res {
        Err(AuthError::MissingCredential) => (),
        _ => panic!("Test gives the result {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "credentials"))]
async fn login_no_user_found(db: PgPool) {
    let res = verify_user_credentials(
        &db,
        "different_user",
        SecretString::new("#strong#_#pass#".to_string()),
    )
    .await;

    match res {
        Err(AuthError::WrongEmailOrPassword) => (),
        _ => panic!("Test gives the result {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "credentials"))]
async fn login_wrong_password(db: PgPool) {
    let res = verify_user_credentials(
        &db,
        "some_user",
        SecretString::new("#wrong#_#pass#".to_string()),
    )
    .await;

    match res {
        Err(AuthError::WrongEmailOrPassword) => (),
        _ => panic!("Test gives the result {:?}", res),
    }
}

#[sqlx::test]
async fn auth_integration_test(db: PgPool) {
    let app_data = tools::AppData::new(db).await;
    let client = app_data.client();

    let payload = json!({
        "email": format!("User{}@gmail.com", nanoid!(10)),
        "password": format!("#very#_#strong#_#pass#"),
        "username": format!("Chad")
    });

    let res = client
        .post(format!("http://{}/api/auth/register", app_data.addr))
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let res = client
        .post(format!("http://{}/api/auth/validate", app_data.addr))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let res = client
        .post(format!("http://{}/api/auth/logout", app_data.addr))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let res = client
        .post(format!("http://{}/api/auth/validate", app_data.addr))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
