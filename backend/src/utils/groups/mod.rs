pub mod models;

use self::models::*;
use crate::errors::{AppError, DbErrMessage};
use anyhow::Context;
use hyper::StatusCode;
use sqlx::{query, query_as, Acquire, Executor, PgPool, Postgres};
use tracing::debug;
use uuid::Uuid;

pub async fn try_add_user_to_group<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: &Uuid,
    group_id: &Uuid,
) -> Result<(), AppError> {
    let mut transaction = conn.begin().await?;

    let username = query!(
        r#"
            SELECT (username) FROM users
            WHERE id = $1
        "#,
        user_id
    )
    .fetch_one(&mut *transaction)
    .await?
    .username;

    debug!("Adding user '{username}' to group ");
    insert_group_user(&mut *transaction, user_id, &username, group_id)
        .await
        .map_err(|e| {
            DbErrMessage::new(e)
                .unique(StatusCode::BAD_REQUEST, "User already in group")
                .fk(StatusCode::BAD_REQUEST, "User or group does not exist")
        })?;

    transaction.commit().await?;

    Ok(())
}

async fn insert_group_user<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: &Uuid,
    username: &str,
    group_id: &Uuid,
) -> sqlx::Result<()> {
    let mut tr = conn.begin().await?;

    query!(
        r#"
            INSERT INTO group_users (user_id, group_id, nickname, role_type)
            VALUES ($1, $2, $3, 'member')
        "#,
        user_id,
        group_id,
        username
    )
    .execute(&mut *tr)
    .await?;

    Ok(())
}

pub async fn create_group(pool: &PgPool, name: &str, user_id: Uuid) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::exp(
            StatusCode::BAD_REQUEST,
            "Missing one or more group fields",
        ))?;
    }

    let mut transaction = pool.begin().await?;

    let group = insert_group_raw(&mut transaction, name).await?;
    let username = select_username_by_id(&mut transaction, user_id).await?;
    insert_default_roles(&mut transaction, group.id).await.context("Failed to initiate group roles")?;
    insert_group_owner(&mut transaction, user_id, group.id, &username).await.map_err(|e| DbErrMessage::new(e).unique(StatusCode::BAD_REQUEST, "User already in group"))?;

    transaction.commit().await?;

    Ok(())
}

async fn insert_group_raw<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    group_name: &str,
) -> sqlx::Result<Group> {
    let mut tr = conn.begin().await?;
    
    let group = query_as!(
        Group,
        r#"
            INSERT INTO groups (name)
            VALUES ($1)
            RETURNING *
        "#,
        group_name
    )
    .fetch_one(&mut *tr)
    .await?;

    Ok(group)
}

async fn select_username_by_id<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
) -> sqlx::Result<String> {
    let mut tr = conn.begin().await?;

    let username = query!(
        r#"
            SELECT (username) FROM users
            WHERE id = $1
        "#,
        user_id,
    )
    .fetch_one(&mut *tr)
    .await?
    .username;

    Ok(username)
}

async fn insert_default_roles<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    group_id: Uuid,
) -> sqlx::Result<()> {
    let mut tr = conn.begin().await?;

    query!(
        r#"
            SELECT add_group_roles($1)
        "#,
        group_id,
    )
    .execute(&mut *tr)
    .await?;

    Ok(())
}

async fn insert_group_owner<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
    group_id: Uuid,
    username: &str,
) -> sqlx::Result<()> {
    let mut tr = conn.begin().await?;

    query!(
        r#"
            INSERT INTO group_users (user_id, group_id, nickname, role_type)
            VALUES ($1, $2, $3, 'owner')
        "#,
        user_id,
        group_id,
        username
    )
    .execute(&mut *tr)
    .await?;

    Ok(())
}

pub async fn check_if_group_member(
    pool: &PgPool,
    user_id: &Uuid,
    group_id: &Uuid,
) -> sqlx::Result<bool> {
    let res = query!(
        r#"
            SELECT EXISTS (
                SELECT 1 FROM group_users
                WHERE user_id = $1 AND group_id = $2
            ) AS "exists!"
        "#,
        user_id,
        group_id
    )
    .fetch_one(pool)
    .await?.exists;

    Ok(res)
}

pub async fn query_user_groups(pool: &PgPool, user_id: &Uuid) -> sqlx::Result<Vec<Group>> {
    let groups = query_as!(
        Group,
        r#"
            SELECT groups.id, groups.name FROM group_users
            JOIN groups ON groups.id = group_users.group_id
            WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(groups)
}

pub async fn check_if_group_exists<'c>(
    exe: impl Executor<'c, Database = Postgres>,
    group_id: &Uuid,
) -> sqlx::Result<bool> {
    let res = query!(
        r#"
            SELECT * FROM groups
            WHERE id = $1
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
) -> sqlx::Result<bool> {
    let res = query!(
        r#"
            SELECT id FROM users
            WHERE id = $1
        "#,
        user_id
    )
    .fetch_optional(exe)
    .await?;

    Ok(res.is_some())
}

pub async fn get_group_info(pool: &PgPool, group_id: &Uuid) -> Result<GroupInfo, AppError> {
    let res = query!(
        r#"
            SELECT g.name,count(user_id) FROM group_users
            JOIN groups g ON group_users.group_id = g.id
            WHERE group_id = $1
            GROUP BY g.name
        "#,
        group_id
    )
    .fetch_optional(pool)
    .await?.ok_or(AppError::exp(
        StatusCode::BAD_REQUEST,
        "Group does not exist",
    ))?;

    Ok(GroupInfo {
        name: res.name,
        members: res.count.unwrap_or(0) as i32,
    })
}

pub async fn try_remove_user_from_group(
    pool: &PgPool,
    user_id: Uuid,
    group_id: Uuid,
) -> sqlx::Result<()> {
    let _ = query!(
        r#"
            DELETE FROM group_users
            WHERE user_id = $1 AND group_id = $2
        "#,
        user_id,
        group_id
    )
    .execute(pool)
    .await?;

    Ok(())
}
