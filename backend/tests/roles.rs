use backend::utils::roles::models::{GroupRolePrivileges, Role};
use backend::utils::roles::models::{
    PrivilegeChangeData, PrivilegeInterpretationData, SocketGroupRolePrivileges, UserRoleChangeData,
};
use backend::utils::roles::privileges::{CanInvite, CanSendMessages, Privilege, Privileges};
use backend::utils::roles::{
    get_group_role_privileges, get_user_role, single_set_group_role_privileges,
    single_set_group_user_role,
};
use sqlx::{query, PgPool};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, PartialEq)]
struct RoleData {
    role: Role,
    privileges: Privileges,
}

const ADIMAC_ID: &str = "ba34ff10-4b89-44cb-9b36-31eb57c41556";
const HUBERT_ID: &str = "263541a8-fa1e-4f13-9e5d-5b250a5a71e6";
const MARCO_ID: &str = "4bd30a6a-7dfe-46a2-b741-f49612aa85c1";
const POLO_ID: &str = "6666e44f-14ce-4aa5-b5f9-8a4cc5ee5c58";

#[sqlx::test(fixtures("groups", "roles", "group_roles"))]
async fn get_group_role_privileges_health_check(db: PgPool) {
    // Hard working rust programmers
    let res = get_group_role_privileges(
        &db,
        Uuid::parse_str("a1fd5c51-326f-476e-a4f7-2e61a692bb56").unwrap(),
    )
    .await
    .expect("Query failed");

    assert_eq!(
        res,
        GroupRolePrivileges(HashMap::from([
            (
                Role::Admin,
                Privileges(HashSet::from([
                    Privilege::CanInvite(CanInvite::Yes),
                    Privilege::CanSendMessages(CanSendMessages::Yes(2)),
                ]))
            ),
            (
                Role::Member,
                Privileges(HashSet::from([
                    Privilege::CanInvite(CanInvite::No),
                    Privilege::CanSendMessages(CanSendMessages::Yes(10)),
                ]))
            ),
        ]))
    )
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_roles", "group_users"))]
async fn get_user_role_health_check(db: PgPool) {
    // Hubert - Chadders
    let res = get_user_role(
        &db,
        &Uuid::parse_str(HUBERT_ID).unwrap(),
        &Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap(),
    )
    .await
    .expect("Query failed");

    assert_eq!(res, Role::Admin)
}

// #[tokio::test]
// async fn preprocess_health_check() {
//     let group_id = Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap();

//     let mut data = GroupUsersRole::from((
//         group_id,
//         [(
//             Role::Admin,
//             vec![Uuid::parse_str(MARCO_ID).unwrap()],
//         )],
//     ));

//     let res = data.preprocess(
//         Role::Admin,
//         Uuid::parse_str(HUBERT_ID).unwrap(),
//     );

//     assert!(res.is_ok());
//     assert_eq!(
//         data,
//         GroupUsersRole::from((
//             group_id,
//             [(
//                 Role::Admin,
//                 vec![Uuid::parse_str(MARCO_ID).unwrap()],
//             )],
//         ))
//     )
// }

// #[tokio::test]
// async fn preprocess_owner_gives_1_owner() {
//     let group_id = Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap();

//     let mut data = GroupUsersRole::from((
//         group_id,
//         [(
//             Role::Owner,
//             vec![Uuid::parse_str(MARCO_ID).unwrap()],
//         )],
//     ));

//     let res = data.preprocess(
//         Role::Owner,
//         Uuid::parse_str(ADIMAC_ID).unwrap(),
//     );

//     assert!(res.is_ok());
//     assert_eq!(
//         data,
//         GroupUsersRole::from((
//             group_id,
//             [(Role::Owner,
//                 vec![Uuid::parse_str(MARCO_ID).unwrap()]
//             ),
//             (Role::Admin,
//                 vec![Uuid::parse_str(ADIMAC_ID).unwrap()]
//             )]
//         ))
//     )
// }

// #[tokio::test]
// async fn preprocess_owner_gives_2_owners() {
//     let group_id = Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap();

//     let mut data = GroupUsersRole::from((
//         group_id,
//         [(Role::Owner,
//         vec![
//             Uuid::parse_str(MARCO_ID).unwrap(),
//             Uuid::parse_str(POLO_ID).unwrap(),
//         ])],
//     ));

//     let res = data.preprocess(
//         Role::Owner,
//         Uuid::parse_str(ADIMAC_ID).unwrap(),
//     );

//     assert!(res.is_err());
// }

// #[tokio::test]
// async fn preprocess_admin_gives_1_owner() {
//     let group_id = Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap();

//     let mut data = GroupUsersRole::from((
//         group_id,
//         [(Role::Owner,
//         vec![Uuid::parse_str(MARCO_ID).unwrap()])],
//     ));

//     let res = data.preprocess(
//         Role::Admin,
//         Uuid::parse_str(HUBERT_ID).unwrap(),
//     );

//     assert!(res.is_err());
// }

// #[tokio::test]
// async fn preprocess_member_changes_role() {
//     let group_id = Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap();

//     let mut data = GroupUsersRole::from((
//         group_id,
//         [(Role::Admin,
//         vec![Uuid::parse_str(MARCO_ID).unwrap()])],
//     ));

//     let res = data.preprocess(
//         Role::Member,
//         Uuid::parse_str(POLO_ID).unwrap(),
//     );

//     assert!(res.is_err());
// }

// #[tokio::test]
// async fn preprocess_self_role() {
//     let group_id = Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap();

//     let mut data = GroupUsersRole::from((
//         group_id,
//         [(Role::Owner,
//         vec![Uuid::parse_str(ADIMAC_ID).unwrap(),])],
//     ));

//     let res = data.preprocess(
//         Role::Owner,
//         Uuid::parse_str(ADIMAC_ID).unwrap(),
//     );

//     assert!(res.is_ok());
//     assert_eq!(data, GroupUsersRole::from((group_id, [])));
// }

#[tokio::test]
async fn maintain_hierarchy_health_check() {
    let old_privileges = SocketGroupRolePrivileges::from(GroupRolePrivileges(HashMap::from([
        (
            Role::Admin,
            Privileges(HashSet::from([
                Privilege::CanInvite(CanInvite::Yes),
                Privilege::CanSendMessages(CanSendMessages::Yes(5)),
            ])),
        ),
        (
            Role::Member,
            Privileges(HashSet::from([
                Privilege::CanInvite(CanInvite::No),
                Privilege::CanSendMessages(CanSendMessages::Yes(10)),
            ])),
        ),
    ])));

    let random_group_id = Uuid::new_v4();
    let mut new_privileges = PrivilegeChangeData::new(
        random_group_id,
        Role::Admin,
        Privilege::CanSendMessages(CanSendMessages::Yes(15)),
    );

    new_privileges
        .maintain_hierarchy(&old_privileges)
        .await
        .unwrap();

    assert_eq!(
        new_privileges,
        PrivilegeChangeData::new(
            random_group_id,
            Role::Admin,
            Privilege::CanSendMessages(CanSendMessages::Yes(10)),
        ),
    );
}

#[sqlx::test(fixtures("users", "groups", "roles", "group_roles"))]
async fn single_set_group_role_privileges_health_check(db: PgPool) {
    let data = PrivilegeChangeData {
        group_id: Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap(),
        role: Role::Member,
        value: Privilege::CanInvite(CanInvite::No),
    };

    // let old_privileges = SocketGroupRolePrivileges ( HashMap::from([
    //     (
    //         Role::Admin,
    //         Arc::new(RwLock::new(Privileges(HashSet::from([
    //             Privilege::CanInvite(CanInvite::Yes),
    //             Privilege::CanSendMessages(CanSendMessages::Yes(5)),
    //         ])))),
    //     ),
    //     (
    //         Role::Member,
    //         Arc::new(RwLock::new(Privileges(HashSet::from([
    //             Privilege::CanInvite(CanInvite::Yes),
    //             Privilege::CanSendMessages(CanSendMessages::Yes(10)),
    //         ])))),
    //     ),
    // ]));

    // data.maintain_hierarchy(&old_privileges).await.unwrap();
    single_set_group_role_privileges(&db, &data).await.unwrap();

    let query_res = query!(
        r#"
            select roles.can_invite, roles.can_send_messages
                from group_roles join roles on group_roles.role_id = roles.id
                where group_roles.group_id = $1
                and group_roles.role_type = $2
        "#,
        data.group_id,
        data.role as Role,
    )
    .fetch_one(&db)
    .await
    .unwrap();

    let res = Privileges::try_from(PrivilegeInterpretationData::new(
        query_res.can_invite,
        query_res.can_send_messages,
    ))
    .unwrap();
    assert_eq!(
        res,
        Privileges::from([
            Privilege::CanInvite(CanInvite::No),
            Privilege::CanSendMessages(CanSendMessages::Yes(10)),
        ])
    )
}

// #[sqlx::test(fixtures("users", "groups", "roles", "group_roles"))]
// async fn single_set_group_role_privileges_with_hierarchy(db: PgPool) {
//     let mut data = PrivilegeChangeData {
//         group_id: Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap(),
//         role: Role::Admin,
//         privilege: PrivilegeType::CanInvite,
//         value: Privilege::CanInvite(CanInvite::No),
//     };

//     let old_privileges = SocketGroupRolePrivileges ( HashMap::from([
//         (
//             Role::Admin,
//             Arc::new(RwLock::new(Privileges(HashMap::from([
//                 (PrivilegeType::CanInvite, Privilege::CanInvite(CanInvite::Yes)),
//                 (PrivilegeType::CanSendMessages, Privilege::CanSendMessages(CanSendMessages::Yes(2))),
//             ])))),
//         ),
//         (
//             Role::Member,
//             Arc::new(RwLock::new(Privileges(HashMap::from([
//                 (PrivilegeType::CanInvite, Privilege::CanInvite(CanInvite::Yes)),
//                 (PrivilegeType::CanSendMessages, Privilege::CanSendMessages(CanSendMessages::Yes(10))),
//             ])))),
//         ),
//     ]));

//     data.maintain_hierarchy(&old_privileges).await.unwrap();
//     single_set_group_role_privileges(&db, &data).await.unwrap();

//     let query_res = query!(
//         r#"
//             select roles.privileges
//                 from group_roles join roles on group_roles.role_id = roles.id
//                 where group_roles.group_id = $1
//                 and group_roles.role_type = $2
//         "#,
//         data.group_id,
//         data.role as Role,
//     )
//     .fetch_one(&db)
//     .await
//     .unwrap();

//     let res: Privileges = serde_json::from_value(query_res.privileges).unwrap();
//     assert_eq!(
//         res,
//         // the change should not happen
//         Privileges(HashMap::from([
//             (PrivilegeType::CanInvite, Privilege::CanInvite(CanInvite::Yes)),
//             (PrivilegeType::CanSendMessages, Privilege::CanSendMessages(CanSendMessages::Yes(2))),
//         ]))
//     )
// }

#[sqlx::test(fixtures("users", "groups", "roles", "group_roles", "group_users"))]
async fn single_set_group_user_role_health_check(db: PgPool) {
    // Chadders - Marco gets Admin
    let data = UserRoleChangeData {
        group_id: Uuid::parse_str("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap(),
        user_id: Uuid::parse_str(MARCO_ID).unwrap(),
        value: Role::Admin,
    };

    single_set_group_user_role(&db, &data).await.unwrap();

    let query_res = query!(
        r#"
            select group_users.user_id, group_roles.role_type as "role: Role" from
            group_users join group_roles on group_users.role_id = group_roles.role_id
            where group_users.group_id = $1
            and group_users.user_id = $2
        "#,
        data.group_id,
        data.user_id,
    )
    .fetch_one(&db)
    .await
    .unwrap();

    assert_eq!(
        (query_res.user_id, query_res.role),
        (Uuid::parse_str(MARCO_ID).unwrap(), Role::Admin),
    )
}
