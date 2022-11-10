use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use anyhow::Context;
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::{query, query_as, Pool, Postgres};
use thiserror::Error;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tracing::debug;
use uuid::Uuid;

use crate::{
    models::{Group, MessageModel},
    queries::AppError,
};

#[derive(Error, Debug)]
pub enum ChatError {
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for ChatError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            ChatError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let info = match self {
            ChatError::Unexpected(_) => "Unexpected server error".into(),
            _ => format!("{self:?}"),
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}

pub struct ChatState {
    pub groups: Mutex<HashMap<Uuid, GroupTransmitter>>,
}

pub struct GroupTransmitter {
    pub tx: broadcast::Sender<String>,
    pub users: HashSet<Uuid>,
}

impl GroupTransmitter {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            tx,
            users: HashSet::new(),
        }
    }
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            groups: Mutex::new(HashMap::new()),
        }
    }
}

pub async fn select_user_groups(
    pool: &Pool<Postgres>,
    user_id: &Uuid,
) -> Result<Vec<Group>, AppError> {
    let res = query_as!(
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
    .context("Failed to select user groups")?;
    Ok(res)
}

pub async fn check_if_is_group_member(
    pool: &Pool<Postgres>,
    group_id: &Uuid,
    user_id: &Uuid,
) -> Result<bool, ChatError> {
    let res = query!(
        r#"
        select * from group_users
        where group_id = $1
        and user_id = $2
    "#,
        group_id,
        user_id
    )
    .fetch_optional(pool)
    .await
    .context("Selecting group user failed")?;

    Ok(res.is_some())
}

pub fn subscribe(
    groups: &mut HashMap<Uuid, GroupTransmitter>,
    group_id: Uuid,
    user_id: Uuid,
    username: &String,
) -> (Sender<String>, Receiver<String>) {
    let group = groups
        .entry(group_id)
        .and_modify(|val| {
            val.users.insert(user_id);
        })
        .or_insert(GroupTransmitter::new());

    let rx = group.tx.subscribe();

    // Send joined message to all subscribers.
    let msg = format!("{} joined.", username);
    debug!("{}", msg);
    let _ = group.tx.send(msg);
    (group.tx.clone(), rx)
}

pub async fn get_user_login_by_id(
    pool: &Pool<Postgres>,
    user_id: &Uuid,
) -> Result<String, ChatError> {
    let res = query!(
        r#"
            select login from users where id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Cannot fetch user login from database")?;

    Ok(res.login)
}

pub async fn fetch_chat_messages(
    pool: &Pool<Postgres>,
    group_id: &Uuid,
) -> Result<Vec<MessageModel>, ChatError> {
    let res = query_as!(
        MessageModel,
        r#"
            select * from messages
            where group_id = $1
        "#,
        group_id
    )
    .fetch_all(pool)
    .await
    .context("Failed to query all messages from group")?;

    Ok(res)
}

pub async fn check_if_group_exists(
    pool: &Pool<Postgres>,
    group_id: &Uuid,
) -> Result<bool, ChatError> {
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

#[cfg(test)]
mod test {
    use sqlx::{query, PgPool, Pool, Postgres};
    #[sqlx::test]
    async fn select_user_groups(pool: Pool<Postgres>) {
        let name = String::from("abc");
        let res = query!(
            r#"
                insert into groups (name)
                values ($1)
                returning id
            "#,
            name
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        // let res = query!(
        //     r#"
        //         insert into group_users (user_id, group_id)
        //         values ($1, $2)
        //     "#,
        //     user_id,
        //     group_id
        // );
    }
}
