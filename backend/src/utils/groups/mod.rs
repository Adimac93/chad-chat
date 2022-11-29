pub mod errors;
pub mod invites;

use crate::models::{Group, GroupInfo};
use anyhow::Context;
use axum::Json;
use errors::*;
use serde_json::{json, Value};
use sqlx::{query, query_as, PgPool};
use uuid::Uuid;

pub async fn try_add_user_to_group(
    pool: &PgPool,
    user_id: &Uuid,
    group_id: &Uuid,
) -> Result<(), GroupError> {
    let mut transaction = pool.begin().await.context("Failed to begin transaction")?;

    let res = query!(
        r#"
        select * from group_users 
        where user_id = $1 and group_id = $2
    "#,
        user_id,
        group_id
    )
    .fetch_optional(&mut transaction)
    .await
    .context("Failed to select group user")?;

    if res.is_some() {
        transaction
            .rollback()
            .await
            .context("Failed when aborting transaction")?;
        return Err(GroupError::UserAlreadyInGroup);
    }

    if !check_if_group_exists(pool, group_id).await? {
        transaction
            .rollback()
            .await
            .context("Failed when aborting transaction")?;
        return Err(GroupError::GroupDoesNotExist);
    }

    if !check_if_user_exists(pool, user_id).await? {
        transaction
            .rollback()
            .await
            .context("Failed when aborting transaction")?;
        return Err(GroupError::UserDoesNotExist);
    }
    
    query!(
        r#"
            insert into group_users (user_id, group_id)
            values ($1, $2)
        "#,
        user_id,
        group_id
    )
    .execute(&mut transaction)
    .await
    .context("Failed to add user to group")?;

    transaction.commit().await.context("Transaction failed")?;

    Ok(())
}

pub async fn create_group(pool: &PgPool, name: &str, user_id: Uuid) -> Result<(), GroupError> {
    if name.trim().is_empty() {
        return Err(GroupError::MissingGroupField);
    }

    let mut transaction = pool.begin().await.context("Failed to create transaction")?;

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
    .await
    .context("Failed to create a group")?;

    query!(
        r#"
            insert into group_users (user_id, group_id)
            values ($1, $2)
        "#,
        user_id,
        group.id
    )
    .execute(&mut transaction)
    .await
    .map_err(|_| GroupError::UserAlreadyInGroup)?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;

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
    .await
    .context("Failed to check if user is in group")?;

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
    .await
    .context("Failed to select groups with provided user id")?;

    Ok(Json(json!({ "groups": groups })))
}

pub async fn check_if_group_exists(pool: &PgPool, group_id: &Uuid) -> Result<bool, GroupError> {
    let res = query!(
        r#"
            select * from groups
            where id = $1
        "#,
        group_id
    )
    .fetch_optional(pool)
    .await
    .context("Failed to select group by id")?;

    Ok(res.is_some())
}

pub async fn check_if_user_exists(pool: &PgPool, user_id: &Uuid) -> Result<bool, GroupError> {
    let res = query!(
        r#"
            select * from users
            where id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await
    .context("Failed to select user by id")?;

    Ok(res.is_some())
}

pub async fn get_group_info(pool: &PgPool, group_id: &Uuid) -> Result<GroupInfo, GroupError> {
    if !check_if_group_exists(pool, group_id).await? {
        return Err(GroupError::GroupDoesNotExist);
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
    .await
    .context("Failed to select group infos")?;

    Ok(GroupInfo {
        name: res.name,
        members: res.count.unwrap_or(0),
    })
}
