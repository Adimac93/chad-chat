pub mod errors;
pub mod messages;
pub mod models;

use anyhow::Context;
use errors::*;
use sqlx::{query, PgPool};
use std::collections::HashMap;
use tokio::sync::broadcast::{Receiver, Sender};
use tracing::debug;
use uuid::Uuid;

use self::models::GroupTransmitter;

// pub fn subscribe(
//     groups: &mut HashMap<Uuid, GroupTransmitter>,
//     group_id: Uuid,
//     user_id: Uuid,
//     username: &String,
// ) -> (Sender<String>, Receiver<String>) {
//     let group = groups
//         .entry(group_id)
//         .and_modify(|val| {
//             val.users.insert(user_id);
//         })
//         .or_insert(GroupTransmitter::new());

//     let rx = group.tx.subscribe();

//     // Send joined message to all subscribers.
//     let msg = format!("{} joined.", username);
//     debug!("{}", msg);
//     let _ = group.tx.send(msg);
//     (group.tx.clone(), rx)
// }

pub async fn get_group_nickname(
    pool: &PgPool,
    user_id: &Uuid,
    group_id: &Uuid,
) -> Result<String, ChatError> {
    let res = query!(
        r#"
            select nickname from group_users
            where user_id = $1 and group_id = $2
        "#,
        user_id,
        group_id
    )
    .fetch_one(pool)
    .await
    .context("Cannot fetch user nickname from database")?;

    Ok(res.nickname)
}

pub async fn get_user_login_by_id(pool: &PgPool, user_id: &Uuid) -> Result<String, ChatError> {
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

pub async fn create_message(
    pool: &PgPool,
    user_id: &Uuid,
    group_id: &Uuid,
    content: &str,
) -> Result<(), ChatError> {
    if content.trim().is_empty() {
        return Err(ChatError::EmptyMessage);
    }

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
    use sqlx::{query, PgPool};
    #[sqlx::test]
    async fn select_user_groups(pool: PgPool) {
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
    }
}
