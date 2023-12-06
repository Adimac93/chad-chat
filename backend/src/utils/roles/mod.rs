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
        UserRoleChangeInput, GroupPrivileges, PrivilegesNumber,
    };

const ROLES_COUNT: usize = 3;
const CACHE_DURATION_IN_SECS: usize = 900;

pub async fn set_privileges<'c>(
    pg: &PgPool,
    rd: &mut RdPool,
    user_id: Uuid,
    data: &PrivilegeChangeInput,
) -> Result<(), AppError> {
    let user_role = get_user_role(pg, rd, user_id, data.group_id).await?;
    if data.role >= user_role {
        return Err(AppError::exp(StatusCode::FORBIDDEN, &format!("Cannot set privileges of {} as {user_role}", data.role)));
    }

    let privileges = get_group_role_privileges(pg, data.group_id, data.role).await?;
    let new_privileges = privileges.update_with(data.value);

    cache_privileges(rd, new_privileges, data.group_id, data.role).await?;
    let query_res = update_privileges(pg, new_privileges, data.group_id, data.role).await;

    if query_res.is_err() {
        invalidate_privilege_cache(rd, data.group_id).await?;
        query_res?;
    }
    
    Ok(())
}

async fn update_privileges(pg: &PgPool, new_value: PrivilegesNumber, group_id: Uuid, role: Role) -> Result<(), AppError> {
    query!(
        r#"
            UPDATE group_roles
            SET privileges = $1
            WHERE group_id = $2
            AND role_type = $3
        "#,
        new_value.inner as i32,
        group_id,
        role as _,
    ).execute(pg).await?;

    Ok(())
}

pub async fn get_group_role_privileges(
    pg: &PgPool,
    group_id: Uuid,
    role: Role,
) -> Result<PrivilegesNumber, AppError> {
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

    Ok(PrivilegesNumber::new(res))
}

async fn cache_privileges(rd: &mut RdPool, privileges: PrivilegesNumber, group_id: Uuid, role: Role) -> Result<(), AppError> {
    rd.send_packed_command(&Cmd::set_ex(&format!("group:{group_id}:role:{role}"), privileges.inner, CACHE_DURATION_IN_SECS)).await.context("Failed to cache privileges")?;
    Ok(())
}

async fn invalidate_privilege_cache(rd: &mut RdPool, group_id: Uuid) -> Result<(), AppError> {
    let mut pipe = Pipeline::new();

    [Role::Owner, Role::Admin, Role::Member].into_iter().for_each(|role| {
        pipe.add_command(Cmd::del(&format!("group:{group_id}:role:{role}")));
    });

    pipe.query_async(rd).await.context("Failed to invalidate privilege cache")?;

    Ok(())
}

pub async fn get_all_privileges(
    pg: &PgPool,
    rd: &mut RdPool,
    group_id: Uuid,
) -> Result<GroupPrivileges, AppError> {
    let cached_privileges = read_group_cached_privileges(rd, group_id).await?;
    let cache_missed = cached_privileges.is_none();
    let privileges = cached_privileges.unwrap_or(select_all_privileges(pg, group_id).await?);
    
    if cache_missed {
        cache_group_privileges(rd, group_id, privileges.clone()).await?;
    }

    Ok(privileges)
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
    if db_res.len() != ROLES_COUNT {
        return Err(AppError::Unexpected(anyhow!("Insufficient role data for group")));
    }

    Ok(GroupPrivileges {
        privileges: HashMap::from_iter(db_res),
    })
}

/// Caches all privileges in the group for all provided roles.
/// Does not cache any privileges in unspecified roles.
async fn cache_group_privileges(
    rd: &mut RdPool,
    group_id: Uuid,
    privileges: GroupPrivileges,
) -> Result<(), AppError> {
    let mut pipe = Pipeline::new();
    [Role::Owner, Role::Admin, Role::Member].into_iter().for_each(|role| {
        let role_privileges = privileges.privileges.get(&role);
        if role_privileges.is_some() {
            pipe.add_command(Cmd::set_ex(&format!("group:{group_id}:role:{}", role), role_privileges, CACHE_DURATION_IN_SECS));
        }
    });
    pipe.query_async(rd).await.context("Failed to cache group privileges")?;

    Ok(())
}

async fn read_group_cached_privileges(
    rd: &mut RdPool,
    group_id: Uuid,
) -> Result<Option<GroupPrivileges>, AppError> {
    let mut pipe = Pipeline::new();
    let atomic_pipe = pipe.atomic();
    atomic_pipe.add_command(Cmd::get(&format!("group:{group_id}:role:{}", Role::Owner)));
    atomic_pipe.add_command(Cmd::get(&format!("group:{group_id}:role:{}", Role::Admin)));
    atomic_pipe.add_command(Cmd::get(&format!("group:{group_id}:role:{}", Role::Member)));
    let query_res: Value = pipe.query_async(rd).await.map_err(|_| AppError::exp(StatusCode::NOT_FOUND, "Failed to query the Redis cache"))?;
    
    let Some(res) = Vec::<u8>::from_redis_value(&query_res).ok() else {
        return Ok(None);
    };

    if res.len() != ROLES_COUNT {
        Err(AppError::Unexpected(anyhow!("Insufficient role data for group")))
    } else {
        Ok(Some(GroupPrivileges {
            privileges: HashMap::from([(Role::Owner, res[0]), (Role::Admin, res[1]), (Role::Member, res[2])]),
        }))
    }
}

pub async fn set_role(
    pg: &PgPool,
    rd: &mut RdPool,
    user_id: Uuid,
    target_user_id: Uuid,
    data: &UserRoleChangeInput,
) -> Result<(), AppError> {
    let user_role = get_user_role(pg, rd, user_id, data.group_id).await?;
    let target_user_current_role = get_user_role(pg, rd, target_user_id, data.group_id).await?;
    if data.value > user_role || target_user_current_role >= user_role {
        return Err(AppError::exp(StatusCode::FORBIDDEN, &format!("Cannot set role from {target_user_current_role} to {} as {user_role}", data.value)));
    }

    let is_owner = select_role(pg, user_id, data.group_id).await? == Role::Owner;
    let change_owner = is_owner && data.value == Role::Owner;

    let mut pg_tr = pg.begin().await?;
    let mut pipe = Pipeline::new();
    let atomic_pipe = pipe.atomic();
    
    update_role(&mut *pg_tr, data.value, data.group_id, target_user_id).await?;
    cache_user_role(atomic_pipe, target_user_id, data.group_id, data.value);
    if change_owner {
        update_role(&mut *pg_tr, Role::Admin, data.group_id, user_id).await?;
        cache_user_role(atomic_pipe, user_id, data.group_id, Role::Admin);
    }

    atomic_pipe.query_async(rd).await.context("Cache write failed")?;
    pg_tr.commit().await?;

    Ok(())
}

async fn update_role<'c>(
    acq: impl Acquire<'c, Database = Postgres>,
    value: Role,
    group_id: Uuid,
    target_user_id: Uuid,
) -> Result<(), AppError> {
    let mut pg_tr = acq.begin().await?;

    query!(
        r#"
            UPDATE group_users
            SET role_type = $1
            WHERE group_id = $2
            AND user_id = $3
        "#,
        value as _,
        group_id,
        target_user_id,
    ).execute(&mut *pg_tr).await?;
    
    Ok(())
}

fn cache_user_role(
    pipe: &mut Pipeline,
    target_user_id: Uuid,
    group_id: Uuid,
    role: Role,
) {
    pipe.add_command(Cmd::set_ex(&format!("group_id:{group_id}:user:{target_user_id}:role"), role.to_string(), CACHE_DURATION_IN_SECS));
}

pub async fn get_user_role(
    pg: &PgPool,
    rd: &mut RdPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<Role, AppError> {
    let cached_role = read_cached_role(rd, user_id, group_id).await?;
    let role = cached_role.unwrap_or(select_role(pg, user_id, group_id).await?);
    
    if cached_role.is_none() {
        let mut pipe = Pipeline::new();
        cache_user_role(&mut pipe, user_id, group_id, role);
    }

    Ok(role)
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

async fn read_cached_role(
    rd: &mut RdPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<Option<Role>, AppError> {
    let res = rd.send_packed_command(&Cmd::get(&format!("group_id:{group_id}:user:{user_id}:role"))).await.context("Failed to query Redis")?;
    
    Ok(Role::from_redis_value(&res).ok())
}

pub async fn get_user_privileges(
    pg: &PgPool,
    rd: &mut RdPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<PrivilegesNumber, AppError> {
    let cached_privileges = read_cached_user_privileges(rd, user_id, group_id).await?;
    let privileges = cached_privileges.unwrap_or(select_user_privileges(pg, user_id, group_id).await?);

    if cached_privileges.is_none() {
        let role = get_user_role(pg, rd, user_id, group_id).await?;
        cache_privileges(rd, privileges, group_id, role).await?;
    }

    Ok(privileges)
}

async fn select_user_privileges(
    pg: &PgPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<PrivilegesNumber, AppError> {
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
    Ok(PrivilegesNumber::new(privileges))
}

async fn read_cached_user_privileges(
    rd: &mut RdPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<Option<PrivilegesNumber>, AppError> {
    let Some(role) = read_cached_role(rd, user_id, group_id).await? else {
        return Ok(None);
    };

    let privileges = read_cached_privileges_by_role(rd, group_id, role).await?;

    Ok(privileges.map(PrivilegesNumber::new))
}

async fn read_cached_privileges_by_role(
    rd: &mut RdPool,
    group_id: Uuid,
    role: Role,
) -> Result<Option<u8>, AppError> {
    let res = rd.send_packed_command(&Cmd::get(&format!("group:{group_id}:role:{role}"))).await.context("Failed to query Redis")?;
    Ok(u8::from_redis_value(&res).ok())
}
