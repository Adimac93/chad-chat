pub mod errors;
pub mod models;
pub mod privileges;

use sqlx::{query, Acquire, PgPool, Postgres};
use uuid::Uuid;

use self::{
    errors::RoleError,
    models::{
        GroupRolePrivileges, PrivilegeChangeData, PrivilegeInterpretationData, Role,
        UserRoleChangeData,
    },
    privileges::{Privilege, Privileges, QueryPrivilege},
};

pub async fn single_set_group_role_privileges<'c>(
    conn: impl Acquire<'c, Database = Postgres> + std::marker::Send,
    data: &PrivilegeChangeData,
) -> Result<(), RoleError> {
    match data.value {
        Privilege::CanInvite(x) => x.set_privilege(conn, data).await?,
        Privilege::CanSendMessages(x) => x.set_privilege(conn, data).await?,
    };

    Ok(())
}

pub async fn get_group_role_privileges(
    pool: &PgPool,
    group_id: Uuid,
) -> Result<GroupRolePrivileges, RoleError> {
    let query_res = query!(
        r#"
            SELECT group_roles.role_type as "role_type: Role", roles.can_invite, roles.can_send_messages from
                group_roles JOIN roles ON group_roles.role_id = roles.id
                WHERE group_roles.group_id = $1
                AND group_roles.role_type IN ('member', 'admin')
                ORDER BY group_roles.role_type
        "#,
        group_id
    )
    .fetch_all(pool)
    .await?;

    let mut res = GroupRolePrivileges::new();
    for role_data in query_res {
        res.0.insert(
            role_data.role_type,
            Privileges::try_from(PrivilegeInterpretationData {
                can_invite: role_data.can_invite,
                can_send_messages: role_data.can_send_messages,
            })?,
        );
    }

    Ok(res)
}

pub async fn single_set_group_user_role<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    data: &UserRoleChangeData,
) -> Result<(), RoleError> {
    let mut transaction = conn.begin().await?;

    let _res = query!(
        r#"
            UPDATE group_users
            SET role_id = group_roles.role_id
            FROM group_roles
            WHERE group_roles.group_id = $1
            AND group_users.user_id = $2
            AND group_roles.role_type = $3
        "#,
        data.group_id,
        data.user_id,
        data.value as Role,
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(())
}

pub async fn get_user_role(
    pool: &PgPool,
    user_id: &Uuid,
    group_id: &Uuid,
) -> Result<Role, RoleError> {
    let res = query!(
        r#"
            SELECT group_roles.role_type AS "role: Role"
            FROM group_users 
            JOIN roles 
            JOIN group_roles ON roles.id = group_roles.role_id
            ON group_users.role_id = roles.id
            WHERE group_users.user_id = $1
            AND group_users.group_id = $2
        "#,
        user_id,
        group_id,
    )
    .fetch_one(pool)
    .await?;

    Ok(res.role)
}
