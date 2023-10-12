use backend::utils::groups::models::GroupInfo;
use backend::utils::groups::{check_if_group_exists, get_group_info};
use backend::utils::groups::{
    check_if_group_member, create_group, errors::GroupError, query_user_groups,
    try_add_user_to_group,
};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(fixtures("users", "groups", "roles", "group_users", "group_roles"))]
async fn add_user_to_group_health_check(db: PgPool) {
    // tries to add Adam to Giga-chadders
    let res = try_add_user_to_group(
        &db,
        &Uuid::parse_str("ba34ff10-4b89-44cb-9b36-31eb57c41556").unwrap(),
        &Uuid::parse_str("347ac024-f8c9-4450-850f-9d85fb17c957").unwrap(),
    )
    .await;

    match res {
        Ok(_) => (),
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_users"))]
async fn add_user_to_group_user_is_in_group(db: PgPool) {
    // tries to add Adam to Chadders
    let res = try_add_user_to_group(
        &db,
        &Uuid::parse_str("ba34ff10-4b89-44cb-9b36-31eb57c41556").unwrap(),
        &Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap(),
    )
    .await;

    match res {
        Err(GroupError::UserAlreadyInGroup) => (),
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_users"))]
async fn add_user_to_group_group_does_not_exist(db: PgPool) {
    // tries to add Adam to ???
    let res = try_add_user_to_group(
        &db,
        &Uuid::parse_str("ba34ff10-4b89-44cb-9b36-31eb57c41556").unwrap(),
        &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
    )
    .await;

    match res {
        Err(GroupError::GroupDoesNotExist) => (),
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_users"))]
async fn add_user_to_group_user_does_not_exist(db: PgPool) {
    // tries to add ??? to Giga-chadders
    let res = try_add_user_to_group(
        &db,
        &Uuid::parse_str("347ac024-f8c9-4450-850f-9d85fb17c957").unwrap(),
        &Uuid::parse_str("347ac024-f8c9-4450-850f-9d85fb17c957").unwrap(),
    )
    .await;

    match res {
        Err(GroupError::UserDoesNotExist) => (),
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_users"))]
async fn create_group_health_check(db: PgPool) {
    let res = create_group(
        &db,
        "Full-Release Males",
        Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
    )
    .await;

    match res {
        Ok(_) => (),
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_users"))]
async fn create_group_missing_group_name(db: PgPool) {
    let res = create_group(
        &db,
        "  ",
        Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
    )
    .await;

    match res {
        Err(GroupError::MissingGroupField) => (),
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_users"))]
async fn check_if_group_member_health_check(db: PgPool) {
    // is Hubert in Chadders?
    let res = check_if_group_member(
        &db,
        &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
        &Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap(),
    )
    .await;

    match res {
        Ok(true) => (),
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_users"))]
async fn check_if_group_member_negative(db: PgPool) {
    // Is Hubert in Indefinable JavaScript undefiners?
    let res = check_if_group_member(
        &db,
        &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
        &Uuid::parse_str("b9ad636d-1163-4d32-8e88-8fb2318468c4").unwrap(),
    )
    .await;

    match res {
        Ok(false) => (),
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_users"))]
async fn query_user_groups_health_check(db: PgPool) {
    // Adam's groups
    let res = query_user_groups(
        &db,
        &Uuid::parse_str("ba34ff10-4b89-44cb-9b36-31eb57c41556").unwrap(),
    )
    .await;

    match res {
        Ok(json) => {
            let objects = json["groups"].as_array().unwrap();
            let mut result_vec: Vec<String> = Vec::new();
            for elem in objects {
                let Value::String(string_enum_val) = elem.get("id").unwrap() else {
                    panic!()
                };
                result_vec.push(string_enum_val.to_string());
            }
            result_vec.sort();
            assert_eq!(
                result_vec,
                vec![
                    "a1fd5c51-326f-476e-a4f7-2e61a692bb56",
                    "b8c9a317-a456-458f-af88-01d99633f8e2"
                ]
            );
        }
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_users"))]
async fn check_if_group_exists_health_check(db: PgPool) {
    // does group "Indefinable JavaScript undefiners" exist?
    let res = check_if_group_exists(
        &db,
        &Uuid::parse_str("b9ad636d-1163-4d32-8e88-8fb2318468c4").unwrap(),
    )
    .await;

    match res {
        Ok(true) => (),
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_users"))]
async fn check_if_group_exists_negative(db: PgPool) {
    // does group ??? exist?
    let res = check_if_group_exists(
        &db,
        &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
    )
    .await;

    match res {
        Ok(false) => (),
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_users"))]
async fn get_group_info_health_check(db: PgPool) {
    // Chadders group info
    let res = get_group_info(
        &db,
        &Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap(),
    )
    .await;

    match res {
        Ok(info)
            if info
                == GroupInfo {
                    members: 4,
                    name: "Chadders".to_string(),
                } =>
        {
            ()
        }
        _ => panic!("Test result is {:?}", res),
    }
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_users"))]
async fn get_group_info_group_does_not_exist(db: PgPool) {
    // ??? group info
    let res = get_group_info(
        &db,
        &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
    )
    .await;

    match res {
        Err(GroupError::GroupDoesNotExist) => (),
        _ => panic!("Test result is {:?}", res),
    }
}
