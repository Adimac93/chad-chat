use anyhow::Context;
use axum::Json;
use serde_json::{Value, json};
use sqlx::{query, Postgres, Pool, query_as};
use uuid::Uuid;
use crate::{
    errors::GroupError,
    models::Group,
};

pub async fn try_add_user_to_group(pool: &Pool<Postgres>, user_id: &Uuid, group_id: &Uuid) -> Result<(), GroupError> {
    let mut transaction = pool.begin().await.context("Failed to begin transaction")?;
    
    // queries for an instance of a particular user in the particular group
    let res = query!(
    r#"
        select * from group_users 
        where user_id = $1 and group_id = $2
    "#,
    user_id,
    group_id
    ).fetch_optional(&mut transaction)
    .await
    .context("Failed to select group user")?;

    if res.is_some() {
        transaction.rollback().await.context("Failed when aborting transaction")?;
        return Err(GroupError::UserAlreadyInGroup);
    }

    // adds the user with a corresponding id to the db
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

pub async fn create_group(
    pool: &Pool<Postgres>,
    name: &str,
    user_id: Uuid,
) -> Result<(), GroupError> {
    if name.is_empty() {
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

pub async fn check_if_group_member(pool: &Pool<Postgres>, user_id: &Uuid, group_id: &Uuid) -> Result<bool, GroupError> {
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

pub async fn query_user_groups(
    pool: &Pool<Postgres>,
    user_id: Uuid,
) -> Result<Json<Value>, GroupError> {
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

pub async fn check_if_group_exists(
    pool: &Pool<Postgres>,
    group_id: &Uuid,
) -> Result<bool, GroupError> {
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
