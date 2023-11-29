pub mod models;

use std::collections::HashMap;

use anyhow::{anyhow, Context};
use hyper::StatusCode;
use redis::{Cmd, Pipeline, Value, FromRedisValue};
use sqlx::{Acquire, PgPool, Postgres, query};
use uuid::Uuid;

use crate::{errors::AppError, state::RdPool};
use self::models::{
        PrivilegeChangeInput, Role,
        UserRoleChangeInput, UserPrivileges, GroupPrivileges,
    };

const EDITABLE_ROLES_COUNT: usize = 2;

pub async fn set_privileges<'c>(
    pg: &PgPool,
    data: &PrivilegeChangeInput,
) -> Result<(), AppError> {
    update_privileges(pg, data).await?;
    Ok(())
}

async fn update_privileges(pg: &PgPool, data: &PrivilegeChangeInput) -> Result<(), AppError> {
    let privileges = get_group_role_privileges(pg, data.group_id, data.role).await?;
    let (target_bits, updated_bits) = data.value.to_bits();

    let target_privileges = ((privileges ^ target_bits) & updated_bits) ^ privileges;

    query!(
        r#"
            UPDATE group_roles
            SET privileges = $1
            WHERE group_id = $2
            AND role_type = $3
        "#,
        target_privileges as i32,
        data.group_id,
        data.role as _,
    ).execute(pg).await?;

    Ok(())
}

async fn cache_privileges(rd: &mut RdPool, group_id: Uuid, role: Role, privileges: u8) -> Result<(), AppError> {
    rd.send_packed_command(&Cmd::set(&format!("group:{group_id}:role:{role}"), privileges)).await.context("Failed to cache privileges")?;
    Ok(())
}

pub async fn get_all_privileges(
    pg: &PgPool,
    rd: &mut RdPool,
    group_id: Uuid,
) -> Result<GroupPrivileges, AppError> {
    let res = read_cached_privileges(rd, group_id).await;

    if res.is_ok() {
        res
    } else {
        Ok(select_all_privileges(pg, group_id).await?)
    }
}

async fn select_all_privileges(
    pg: &PgPool,
    group_id: Uuid,
) -> Result<GroupPrivileges, AppError> {
    let query_res = select_all_privileges_raw(pg, group_id).await?;
    Ok(map_privileges(query_res)?)
}

async fn select_all_privileges_raw(
    pg: &PgPool,
    group_id: Uuid,
) -> Result<Vec<(Role, u8)>, AppError> {
    let query_res = query!(
        r#"
            SELECT role_type as "role: Role", privileges
            FROM group_roles
            WHERE group_id = $1
        "#,
        group_id
    ).fetch_all(pg).await?;

    let res: Vec<(Role, u8)> = query_res.into_iter().map(|x| {
        let privileges: u8 = x.privileges.try_into().context("Failed to process privileges")?;
        Result::<(Role, u8), AppError>::Ok((x.role, privileges))
    }).collect::<Result<_, AppError>>()?;

    Ok(res)
}

fn map_privileges(
    db_res: Vec<(Role, u8)>,
) -> Result<GroupPrivileges, AppError> {
    if db_res.len() != EDITABLE_ROLES_COUNT {
        return Err(AppError::Unexpected(anyhow!("Insufficient role data for group")));
    }

    let mut res = GroupPrivileges {
        privileges: HashMap::new()
    };

    for i in 0..EDITABLE_ROLES_COUNT {
        if db_res[i].0 == Role::Owner {
            return Err(AppError::Unexpected(anyhow!("Invalid role found during receiving privilege data")))
        } else {
            res.privileges.insert(db_res[i].0, db_res[i].1);
        }
    }

    Ok(res)
}

async fn read_cached_privileges(
    rd: &mut RdPool,
    group_id: Uuid,
) -> Result<GroupPrivileges, AppError> {
    let mut pipe = Pipeline::new();
    let atomic_pipe = pipe.atomic();
    atomic_pipe.add_command(Cmd::get(&format!("group:{group_id}:role:{}", Role::Admin)));
    atomic_pipe.add_command(Cmd::get(&format!("group:{group_id}:role:{}", Role::Member)));
    let query_res: Value = pipe.query_async(rd).await.map_err(|_| AppError::exp(StatusCode::NOT_FOUND, "Failed to query the Redis cache"))?;
    
    let res: Vec<u8> = Vec::<u8>::from_redis_value(&query_res).map_err(|_| AppError::exp(StatusCode::NOT_FOUND, "Cache missed"))?;

    if res.len() != EDITABLE_ROLES_COUNT {
        Err(AppError::Unexpected(anyhow!("Insufficient role data for group")))
    } else {
        Ok(GroupPrivileges {
            privileges: HashMap::from([(Role::Admin, res[0]), (Role::Member, res[1])]),
        })
    }
}

pub async fn get_group_role_privileges(
    pg: &PgPool,
    group_id: Uuid,
    role: Role,
) -> Result<u8, AppError> {
    let query_res = query!(
        r#"
            SELECT privileges
            FROM group_roles
            WHERE group_id = $1
            AND role_type = $2
        "#,
        group_id,
        role as _,
    ).fetch_one(pg).await?;

    let res: u8 = query_res.privileges.try_into().map_err(|_| AppError::Unexpected(anyhow!("Failed to retrieve the privileges as u8")))?;

    Ok(res)
}

pub async fn set_role<'c>(
    acq: impl Acquire<'c, Database = Postgres>,
    data: &UserRoleChangeInput,
) -> Result<(), AppError> {
    let mut pg_tr = acq.begin().await?;

    update_role(&mut *pg_tr, data).await?;

    Ok(())
}

async fn update_role<'c>(
    acq: impl Acquire<'c, Database = Postgres>,
    data: &UserRoleChangeInput,
) -> Result<(), AppError> {
    let mut pg_tr = acq.begin().await?;

    query!(
        r#"
            UPDATE group_users
            SET role_type = $1
            WHERE group_id = $2
            AND user_id = $3
        "#,
        data.value as _,
        data.group_id,
        data.user_id,
    ).execute(&mut *pg_tr).await?;
    
    Ok(())
}

pub async fn get_user_role(
    pg: &PgPool,
    rd: &mut RdPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<Role, AppError> {
    let res = read_cached_role(rd, user_id, group_id).await;

    if res.is_ok() {
        res
    } else {
        Ok(select_role(pg, user_id, group_id).await?)
    }
}

async fn select_role(
    pg: &PgPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<Role, AppError> {
    let res = query!(
        r#"
            SELECT role_type AS "role_type: Role"
            FROM group_users
            WHERE group_id = $1
            AND user_id = $2
        "#,
        group_id,
        user_id,
    ).fetch_one(pg).await?;

    Ok(res.role_type)
}

async fn cache_role(
    rd: &mut RdPool,
    user_id: Uuid,
    group_id: Uuid,
    role: Role,
) -> Result<(), AppError> {
    rd.send_packed_command(&Cmd::set(&format!("group_id:{group_id}:user:{user_id}:role"), role.to_string())).await.context("Failed to cache role")?;
    Ok(())
}

async fn read_cached_role(
    rd: &mut RdPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<Role, AppError> {
    let res = rd.send_packed_command(&Cmd::get(&format!("group_id:{group_id}:user:{user_id}:role"))).await.context("Failed to query Redis")?;
    
    Ok(Role::from_redis_value(&res).map_err(|_| AppError::exp(StatusCode::NOT_FOUND, "Cache missed"))?)
}

pub async fn get_user_privileges(
    pg: &PgPool,
    rd: &mut RdPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<UserPrivileges, AppError> {
    let res = read_cached_user_privileges(rd, user_id, group_id).await;

    if res.is_ok() {
        res
    } else {
        Ok(select_user_privileges(pg, user_id, group_id).await?)
    }
}

async fn select_user_privileges(
    pg: &PgPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<UserPrivileges, AppError> {
    let res = query!(
        r#"
            SELECT privileges
            FROM group_users
            JOIN group_roles ON group_users.role_type = group_roles.role_type
            WHERE user_id = $1
            AND group_users.group_id = $2
        "#,
        user_id,
        group_id
    ).fetch_one(pg).await?;

    let privileges = res.privileges.try_into().context("Failed to process the privileges")?;
    Ok(UserPrivileges { privileges })
}

async fn read_cached_user_privileges(
    rd: &mut RdPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<UserPrivileges, AppError> {
    let role = read_cached_role(rd, user_id, group_id).await?;
    let privileges = read_cached_privileges_by_role(rd, group_id, role).await?;

    Ok(UserPrivileges { privileges })
}

async fn read_cached_privileges_by_role(
    rd: &mut RdPool,
    group_id: Uuid,
    role: Role,
) -> Result<u8, AppError> {
    let res = rd.send_packed_command(&Cmd::get(&format!("group:{group_id}:role:{role}"))).await.context("Failed to query Redis")?;
    Ok(u8::from_redis_value(&res).map_err(|_| AppError::exp(StatusCode::NOT_FOUND, "Cache missed"))?)
}
