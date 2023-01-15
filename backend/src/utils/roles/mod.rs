pub mod errors;
pub mod models;

use sqlx::{query, PgPool};
use uuid::Uuid;

use self::{errors::RoleError, models::{GroupUsersRole, GroupRolePrivileges, Role, BulkNewGroupRolePrivileges}};

use super::groups::models::GroupUser;

pub async fn get_group_role_privileges(pool: &PgPool, group_id: Uuid) -> Result<GroupRolePrivileges, RoleError> {
    let query_res = query!(
        r#"
            select group_roles.role_type as "role_type: Role", roles.privileges from
                group_roles join roles on group_roles.role_id = roles.id
                where group_roles.group_id = $1
                and group_roles.role_type in ('member', 'admin')
                order by group_roles.role_type
        "#,
        group_id
    )
    .fetch_all(pool)
    .await?;

    let mut res = GroupRolePrivileges::new();
    for role_data in query_res {
        res.0.insert(role_data.role_type, serde_json::from_value(role_data.privileges)?);
    }

    Ok(res)
}

pub async fn bulk_set_group_role_privileges(pool: &PgPool, group_id: &Uuid, new_privileges: &BulkNewGroupRolePrivileges) -> Result<(), RoleError> {
    let mut transaction = pool.begin().await?;

    for (role, privileges) in &new_privileges.0 {
        // rollbacks automatically on error
        let _res = query!(
            r#"
                update roles
                    set privileges = $1
                    where roles.id = (
                        select role_id
                            from group_roles
                            where group_roles.role_type = $2
                            and group_roles.group_id = $3
                    )
            "#,
            &serde_json::to_value(&privileges)?,
            &role as &Role,
            &group_id,
        )
        .execute(&mut transaction)
        .await?;
    }

    transaction.commit().await?;

    Ok(())
}

// pub async fn set_group_role_privileges(pool: &PgPool, group_id: &Uuid, new_privileges: &BulkNewGroupRolePrivileges) -> Result<(), RoleError> {
//     let mut transaction = pool.begin().await?;

//     for (role, privileges) in &new_privileges.0 {
//         // rollbacks automatically on error
//         let _res = query!(
//             r#"
//                 update roles
//                     set privileges = $1
//                     where roles.id = (
//                         select role_id
//                             from group_roles
//                             where group_roles.role_type = $2
//                             and group_roles.group_id = $3
//                     )
//             "#,
//             &serde_json::to_value(&privileges)?,
//             &role as &Role,
//             &group_id,
//         )
//         .execute(&mut transaction)
//         .await?;
//     }

//     transaction.commit().await?;

//     Ok(())
// }

pub async fn bulk_set_group_users_role(pool: &PgPool, roles: &GroupUsersRole) -> Result<(), RoleError> {
    let mut transaction = pool.begin().await?;
    
    for (role, users) in &roles.0 {
        // rollbacks automatically on error
        let _res = query!(
            r#"
                update group_users
                set role_id = (
                    select role_id
                        from group_roles
                        where group_roles.group_id = group_users.group_id
                        and group_roles.role_type = $1
                )
                where (user_id, group_id) = any($2)
            "#,
            role as &Role,
            users as &[GroupUser],
        )
        .execute(&mut transaction)
        .await?;
    }

    transaction.commit().await?;

    Ok(())
}

pub async fn get_user_role(pool: &PgPool, user_id: &Uuid, group_id: &Uuid) -> Result<Role, RoleError> {
    let res = query!(
        r#"
            select
                group_roles.role_type as "role: Role"
                from group_users join
                    roles join group_roles on roles.id = group_roles.role_id
                on group_users.role_id = roles.id
                where group_users.user_id = $1
                and group_users.group_id = $2
        "#,
        user_id,
        group_id,
    )
    .fetch_one(pool)
    .await?;

    Ok(res.role)
}
