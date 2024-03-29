pub mod errors;
pub mod messages;
pub mod models;
pub mod socket;

use anyhow::Context;
use errors::*;
use sqlx::{query, PgPool};
use uuid::Uuid;

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

pub async fn get_user_email_by_id(pool: &PgPool, user_id: &Uuid) -> Result<String, ChatError> {
    let res = query!(
        r#"
            select email from credentials where id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Cannot fetch user email from database")?;

    Ok(res.email)
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
