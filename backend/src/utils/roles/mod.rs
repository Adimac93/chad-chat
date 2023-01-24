pub mod errors;
pub mod models;
pub mod privileges;

use sqlx::{query, PgPool, Acquire, Postgres};
use uuid::Uuid;

use self::{errors::RoleError, models::{PrivilegeInterpretationData, GroupUsersRole, GroupRolePrivileges, Role, PrivilegeChangeData, UserRoleChangeData, BulkChangePrivileges, BulkRoleChangeData}, privileges::{QueryPrivilege, Privilege, Privileges}};

pub async fn get_group_role_privileges(pool: &PgPool, group_id: Uuid) -> Result<GroupRolePrivileges, RoleError> {
    let query_res = query!(
        r#"
            select group_roles.role_type as "role_type: Role", roles.can_invite, roles.can_send_messages from
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
        res.0.insert(role_data.role_type, Privileges::try_from(PrivilegeInterpretationData {
            can_invite: role_data.can_invite,
            can_send_messages: role_data.can_send_messages,
        })?);
    }

    Ok(res)
}

pub async fn bulk_set_group_role_privileges(pool: &PgPool, group_id: &Uuid, new_privileges: &BulkChangePrivileges) -> Result<(), RoleError> {
    let mut transaction = pool.begin().await?;

    for data in &new_privileges.0 {
        single_set_group_role_privileges(&mut transaction, data).await?;
    }

    transaction.commit().await?;

    Ok(())
}

pub async fn single_set_group_role_privileges<'c>(
    conn: impl Acquire<'c, Database = Postgres> + std::marker::Send,
    data: &PrivilegeChangeData
) -> Result<(), RoleError> {
    match data.value {
        Privilege::CanInvite(x) => x.set_privilege(conn, data).await?,
        Privilege::CanSendMessages(x) => x.set_privilege(conn, data).await?,
    };

    Ok(())
}

pub async fn bulk_set_group_users_role(pool: &PgPool, roles: &BulkRoleChangeData) -> Result<(), RoleError> {
    let mut transaction = pool.begin().await?;
    
    for data in &roles.0 {
        single_set_group_user_role(&mut transaction, data).await?;
    }

    transaction.commit().await?;

    Ok(())
}

pub async fn single_set_group_user_role<'c>(conn: impl Acquire<'c, Database = Postgres>, data: &UserRoleChangeData) -> Result<(), RoleError> {
    let mut transaction = conn.begin().await?;
    
    let _res = query!(
        r#"
            update group_users
                set role_id = group_roles.role_id
                from group_roles
                where group_roles.group_id = $1
                and group_users.user_id = $2
                and group_roles.role_type = $3
        "#,
        data.group_id,
        data.user_id,
        data.value as Role,
    )
    .execute(&mut transaction)
    .await?;

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
