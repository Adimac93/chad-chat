use crate::{
    configuration::get_config,
    modules::database::{get_redis_pool, DatabaseError, RdPool},
};
use nanoid::nanoid;
use redis::{aio::ConnectionLike, AsyncCommands, Cmd, Pipeline};
use serde::{Deserialize, Serialize};
use sqlx::{pool, query, query_as, Acquire, PgPool, Postgres};
use std::time::Duration;
use tokio::{sync::Notify, task::JoinHandle};
use tracing_subscriber::fmt::format;
use uuid::Uuid;

#[derive(sqlx::Type, Debug, Serialize, Deserialize)]
#[sqlx(type_name = "token_type", rename_all = "snake_case")]
pub enum Token {
    Registration,
    Network,
}

impl ToString for Token {
    fn to_string(&self) -> String {
        match self {
            Token::Registration => "reg".into(),
            Token::Network => "net".into(),
        }
    }
}

impl Token {
    pub async fn use_token<'c>(
        &self,
        rdpool: &mut RdPool,
        token_id: &Uuid,
    ) -> Result<Option<Uuid>, DatabaseError> {
        let key = format!("tokens:{}:{token_id}", self.to_string());
        let res: Option<String> = rdpool.get_del(key).await?;
        if let Some(val) = res {
            let user_id = Uuid::try_parse(&val).unwrap();
            return Ok(Some(user_id));
        }
        Ok(None)
    }

    pub async fn gen_token_with_duration(
        &self,
        rdpool: &mut RdPool,
        user_id: &Uuid,
    ) -> Result<Uuid, DatabaseError> {
        let token_id = Uuid::new_v4();
        let key = format!("tokens:{}:{token_id}", self.to_string());
        rdpool.set_ex(&key, user_id.to_string(), 60 * 15).await?;

        Ok(token_id)
        // redis::cmd("SET")
        //     .arg(&key)
        //     .arg(nanoid!())
        //     .arg("EX")
        //     .arg(60 * 15)
        //     .query_async(rdpool)
        //     .await?;

        // rdpool.set(&key, nanoid!()).await?;
        // rdpool.expire(&key, 60 * 15).await?;

        // rdpool
        //     .req_packed_command(&Cmd::set(&key, nanoid!()).arg("EX").arg(60 * 15))
        //     .await?;

        // rdpool
        //     .req_packed_commands(
        //         &Pipeline::new().set(&key, nanoid!()).expire(&key, 60 * 15),
        //         0,
        //         2,
        //     )
        //     .await?;
    }
}

#[tokio::test]
async fn generate_and_use_reg_token() {
    let config = get_config().unwrap();
    let mut rdpool = get_redis_pool(config.redis).await;

    let token_id = Token::Registration
        .gen_token_with_duration(&mut rdpool, &Uuid::new_v4())
        .await
        .unwrap();

    Token::Registration.use_token(&mut rdpool, &token_id).await;
}
