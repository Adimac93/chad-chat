mod tests {
    use backend::utils::groups::{try_add_user_to_group, errors::GroupError, create_group, check_if_group_member, query_user_groups};
    use serde_json::json;
    use sqlx::PgPool;
    use uuid::Uuid;

    use super::*;

    #[sqlx::test(fixtures("users", "groups", "group_users"))]
    async fn add_user_to_group_positive(db: PgPool) {
        // tries to add Adam to Giga-chadders
        let res = try_add_user_to_group(
            &db,
            &Uuid::parse_str("ba34ff10-4b89-44cb-9b36-31eb57c41556").unwrap(),
            &Uuid::parse_str("347ac024-f8c9-4450-850f-9d85fb17c957").unwrap()
        ).await;

        match res {
            Ok(_) => (),
            _ => panic!("Test result is {:?}", res),
        }
    }

    #[sqlx::test(fixtures("users", "groups", "group_users"))]
    async fn add_user_to_group_user_is_in_group(db: PgPool) {
        // tries to add Adam to Chadders
        let res = try_add_user_to_group(
            &db,
            &Uuid::parse_str("ba34ff10-4b89-44cb-9b36-31eb57c41556").unwrap(),
            &Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap()
        ).await;

        match res {
            Err(GroupError::UserAlreadyInGroup) => (),
            _ => panic!("Test result is {:?}", res),
        }
    }

    #[sqlx::test(fixtures("users", "groups", "group_users"))]
    async fn create_group_positive(db: PgPool) {
        let res = create_group (
            &db,
            "Full-Release Males",
            Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap()
        ).await;

        match res {
            Ok(_) => (),
            _ => panic!("Test result is {:?}", res),
        }
    }

    #[sqlx::test(fixtures("users", "groups", "group_users"))]
    async fn create_group_missing_group_name(db: PgPool) {
        let res = create_group (
            &db,
            "  ",
            Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap()
        ).await;

        match res {
            Err(GroupError::MissingGroupField) => (),
            _ => panic!("Test result is {:?}", res),
        }
    }

    #[sqlx::test(fixtures("users", "groups", "group_users"))]
    async fn check_if_group_member_positive(db: PgPool) {
        // is Hubert in Chadders?
        let res = check_if_group_member(
            &db,
            &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
            &Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap()
        ).await;

        match res {
            Ok(true) => (),
            _ => panic!("Test result is {:?}", res),
        }
    }

    #[sqlx::test(fixtures("users", "groups", "group_users"))]
    async fn check_if_group_member_negative(db: PgPool) {
        // Is Hubert in Indefinable JavaScript undefiners?
        let res = check_if_group_member(
            &db,
            &Uuid::parse_str("263541a8-fa1e-4f13-9e5d-5b250a5a71e6").unwrap(),
            &Uuid::parse_str("b9ad636d-1163-4d32-8e88-8fb2318468c4").unwrap()
        ).await;

        match res {
            Ok(false) => (),
            _ => panic!("Test result is {:?}", res),
        }
    }

    #[sqlx::test(fixtures("users", "groups", "group_users"))]
    async fn query_user_groups_positive(db: PgPool) {
        // Adam's groups
        let res = query_user_groups (
            &db,
            &Uuid::parse_str("ba34ff10-4b89-44cb-9b36-31eb57c41556").unwrap(),
        ).await;

        match res {
            Ok(groups) if groups["groups"].as_array().expect("Invalid data format").len() == 2 => (),
            _ => panic!("Test result is {:?}", res),
        }
    }
}
