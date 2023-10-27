pub mod models;

use self::models::*;
use crate::errors::AppError;
use anyhow::Context;
use axum::Json;
use hyper::StatusCode;
use serde_json::{json, Value};
use sqlx::{query, query_as, Acquire, Executor, PgPool, Postgres};
use tracing::debug;
use uuid::Uuid;

pub async fn try_add_user_to_group<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: &Uuid,
    group_id: &Uuid,
) -> Result<(), AppError> {
    let mut transaction = conn.begin().await?;

    let res = query!(
        r#"
            SELECT * FROM group_users 
            WHERE user_id = $1 AND group_id = $2
        "#,
        user_id,
        group_id
    )
    .fetch_optional(&mut *transaction)
    .await?;

    if res.is_some() {
        transaction.rollback().await?;
        return Err(AppError::exp(
            StatusCode::BAD_REQUEST,
            "User already in group",
        ));
    }

    if !check_if_group_exists(&mut *transaction, group_id).await? {
        transaction.rollback().await?;
        return Err(AppError::exp(
            StatusCode::BAD_REQUEST,
            "Group does not exist",
        ));
    }

    if !check_if_user_exists(&mut *transaction, user_id).await? {
        transaction.rollback().await?;
        return Err(AppError::exp(
            StatusCode::BAD_REQUEST,
            "User does not exist",
        ));
    }

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
    query!(
        r#"
            INSERT INTO group_users (user_id, group_id, nickname, role_id)
            VALUES ($1, $2, $3, (
                SELECT role_id
                FROM group_roles
                WHERE group_roles.group_id = $2
                AND group_roles.role_type = 'member'
            ))
        "#,
        user_id,
        group_id,
        username
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

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

    let group = query_as!(
        Group,
        r#"
            INSERT INTO groups (name)
            VALUES ($1)
            RETURNING *
        "#,
        name
    )
    .fetch_one(&mut *transaction)
    .await?;

    let username = query!(
        r#"
            SELECT (username) FROM users
            WHERE id = $1
        "#,
        user_id,
    )
    .fetch_one(&mut *transaction)
    .await?
    .username;

    query!(
        r#"
            SELECT add_group_roles($1)
        "#,
        group.id,
    )
    .execute(&mut *transaction)
    .await
    .context("Failed to initiate group roles")?;

    query!(
        r#"
            INSERT INTO group_users (user_id, group_id, nickname, role_id)
            VALUES ($1, $2, $3, (
                SELECT role_id
                FROM group_roles
                WHERE group_roles.group_id = $2
                AND group_roles.role_type = 'owner'
            ))
        "#,
        user_id,
        group.id,
        username
    )
    .execute(&mut *transaction)
    .await
    .map_err(|_| AppError::exp(StatusCode::BAD_REQUEST, "User already in group"))?;

    transaction.commit().await?;

    Ok(())
}

pub async fn check_if_group_member(
    pool: &PgPool,
    user_id: &Uuid,
    group_id: &Uuid,
) -> Result<bool, AppError> {
    let res = query!(
        r#"
            SELECT * FROM group_users
            WHERE user_id = $1 AND group_id = $2
        "#,
        user_id,
        group_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(res.is_some())
}

pub async fn query_user_groups(pool: &PgPool, user_id: &Uuid) -> Result<Json<Value>, AppError> {
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

    Ok(Json(json!({ "groups": groups })))
}

pub async fn check_if_group_exists<'c>(
    exe: impl Executor<'c, Database = Postgres>,
    group_id: &Uuid,
) -> Result<bool, AppError> {
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
) -> Result<bool, AppError> {
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
    if !check_if_group_exists(pool, group_id).await? {
        return Err(AppError::exp(
            StatusCode::BAD_REQUEST,
            "Group does not exist",
        ))?;
    }

    let res = query!(
        r#"
            SELECT g.name,count(user_id) FROM group_users
            JOIN groups g ON group_users.group_id = g.id
            WHERE group_id = $1
            GROUP BY g.name
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
) -> Result<(), AppError> {
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
