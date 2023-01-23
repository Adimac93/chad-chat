pub mod errors;
pub mod models;

use self::models::*;
use anyhow::Context;
use axum::Json;
use errors::*;
use serde_json::{json, Value};
use sqlx::{query, query_as, Acquire, Executor, PgPool, Postgres};
use tracing::debug;
use uuid::Uuid;

pub async fn try_add_user_to_group<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: &Uuid,
    group_id: &Uuid,
) -> Result<(), GroupError> {
    let mut transaction = conn.begin().await?;

    let res = query!(
        r#"
            select * from group_users 
            where user_id = $1 and group_id = $2
        "#,
        user_id,
        group_id
    )
    .fetch_optional(&mut transaction)
    .await?;

    if res.is_some() {
        transaction.rollback().await?;
        return Err(GroupError::UserAlreadyInGroup);
    }

    if !check_if_group_exists(&mut transaction, group_id).await? {
        transaction.rollback().await?;
        return Err(GroupError::GroupDoesNotExist);
    }

    if !check_if_user_exists(&mut transaction, user_id).await? {
        transaction.rollback().await?;
        return Err(GroupError::UserDoesNotExist);
    }

    let username = query!(
        r#"
            select (username) from users
            where id = $1
        "#,
        user_id
    )
    .fetch_one(&mut transaction)
    .await?
    .username;

    debug!("Adding user '{username}' to group ");
    query!(
        r#"
            insert into group_users (user_id, group_id, nickname, role_id)
            values ($1, $2, $3, (
                select role_id
                    from group_roles
                    where group_roles.group_id = $2
                    and group_roles.role_type = 'member'
            ))
        "#,
        user_id,
        group_id,
        username
    )
    .execute(&mut transaction)
    .await?;

    transaction.commit().await?;

    Ok(())
}

pub async fn create_group(pool: &PgPool, name: &str, user_id: Uuid) -> Result<(), GroupError> {
    if name.trim().is_empty() {
        return Err(GroupError::MissingGroupField)?;
    }

    let mut transaction = pool.begin().await?;

    let group = query_as!(
        Group,
        r#"
            insert into groups (name)
            values ($1)
            returning *
        "#,
        name
    )
    .fetch_one(&mut transaction)
    .await?;

    let username = query!(
        r#"
            select (username) from users
            where id = $1
        "#,
        user_id,
    )
    .fetch_one(&mut transaction)
    .await?
    .username;

    query!(
        r#"
            select add_group_roles($1)
        "#,
        group.id,
    )
    .execute(&mut transaction)
    .await
    .context("Failed to initiate group roles")?;

    query!(
        r#"
            insert into group_users (user_id, group_id, nickname, role_id)
            values ($1, $2, $3, (
                select role_id
                    from group_roles
                    where group_roles.group_id = $2
                    and group_roles.role_type = 'owner'
            ))
        "#,
        user_id,
        group.id,
        username
    )
    .execute(&mut transaction)
    .await
    .map_err(|_| GroupError::UserAlreadyInGroup)?;

    transaction.commit().await?;

    Ok(())
}

pub async fn check_if_group_member(
    pool: &PgPool,
    user_id: &Uuid,
    group_id: &Uuid,
) -> Result<bool, GroupError> {
    let res = query!(
        r#"
            select * from group_users
            where user_id = $1 and group_id = $2
        "#,
        user_id,
        group_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(res.is_some())
}

pub async fn query_user_groups(pool: &PgPool, user_id: &Uuid) -> Result<Json<Value>, GroupError> {
    let groups = query_as!(
        Group,
        r#"
            select groups.id, groups.name from group_users
            join groups on groups.id = group_users.group_id
            where user_id = $1
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(Json(json!({ "groups": groups })))
}

pub async fn check_if_group_exists<'c>(
    exe: impl Executor<'c, Database = Postgres>,
    group_id: &Uuid,
) -> Result<bool, GroupError> {
    let res = query!(
        r#"
            select * from groups
            where id = $1
        "#,
        group_id
    )
    .fetch_optional(exe)
    .await?;

    Ok(res.is_some())
}

pub async fn check_if_user_exists<'c>(
    exe: impl Executor<'c, Database = Postgres>,
    user_id: &Uuid,
) -> Result<bool, GroupError> {
    let res = query!(
        r#"
            select id from users
            where id = $1
        "#,
        user_id
    )
    .fetch_optional(exe)
    .await?;

    Ok(res.is_some())
}

pub async fn get_group_info(pool: &PgPool, group_id: &Uuid) -> Result<GroupInfo, GroupError> {
    if !check_if_group_exists(pool, group_id).await? {
        return Err(GroupError::GroupDoesNotExist)?;
    }

    let res = query!(
        r#"
            select g.name,count(user_id) from group_users
            join groups g on group_users.group_id = g.id
            where group_id = $1
            group by g.name
        "#,
        group_id
    )
    .fetch_one(pool)
    .await?;

    Ok(GroupInfo {
        name: res.name,
        members: res.count.unwrap_or(0),
    })
}

pub async fn try_remove_user_from_group(
    pool: &PgPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<(), GroupError> {
    let _ = query!(
        r#"
            delete from group_users
            where user_id = $1 and group_id = $2
        "#,
        user_id,
        group_id
    )
    .execute(pool)
    .await?;

    Ok(())
}
