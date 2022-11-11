use std::{
    collections::HashMap,
};

use anyhow::Context;
use sqlx::{query, query_as, Pool, Postgres};
use tokio::sync::broadcast::{Receiver, Sender};
use tracing::debug;
use uuid::Uuid;

use crate::{
    models::{MessageModel, GroupTransmitter},
    errors::ChatError,
};

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

pub async fn create_message(pool: &Pool<Postgres>, user_id: &Uuid, group_id: &Uuid, content: &str) -> Result<(), ChatError> {
    query!(
        r#"
            insert into messages (content, user_id, group_id)
            values ($1, $2, $3)
        "#,
        content,
        user_id,
        group_id
    )
    .execute(pool)
    .await
    .context("Failed to add message")?;
    Ok(())
}

#[cfg(test)]
mod test {
    use sqlx::{query, Pool, Postgres};
    #[sqlx::test]
    async fn select_user_groups(pool: Pool<Postgres>) {
        let name = String::from("abc");
        let _res = query!(
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
