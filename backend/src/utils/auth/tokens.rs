use serde::{Deserialize, Serialize};
use sqlx::{pool, query, query_as, Acquire, PgPool, Postgres};
use std::time::Duration;
use tokio::{sync::Notify, task::JoinHandle};
use uuid::Uuid;

#[derive(sqlx::Type, Debug, Serialize, Deserialize)]
#[sqlx(type_name = "token_type", rename_all = "snake_case")]
pub enum Token {
    Registration,
    Network,
}

impl Token {
    pub async fn gen_token<'c>(&self, pool: &PgPool, user_id: &Uuid) -> Result<Uuid, sqlx::Error> {
        let token_id = query!(
            r#"
                insert into user_tokens (token, user_id)
                values ($1, $2)
                returning (id)
            "#,
            &self as &Token,
            user_id
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(token_id)
    }

    pub async fn use_token<'c>(
        &self,
        acq: impl Acquire<'c, Database = Postgres>,
        user_id: &Uuid,
        token_id: &Uuid,
    ) -> Result<Token, sqlx::Error> {
        let mut conn = acq.acquire().await?;
        let token = query!(
            r#"
            delete from user_tokens
            where id = $1 and user_id = $2
            returning token as "token: Token" 
        "#,
            token_id,
            user_id
        )
        .fetch_one(&mut *conn)
        .await?
        .token;

        match token {
            Token::Registration => todo!(), // change account status to verified
            Token::Network => todo!(),      // add ip address as trusted
        }
    }
}
