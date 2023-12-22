pub mod redis_path;

use std::fmt::Display;
use axum::async_trait;
use redis::{RedisResult, Pipeline, FromRedisValue, ToRedisArgs};
use redis::aio::ConnectionLike;
use redis::{cmd, Cmd, Value};

pub async fn get_at(rd: &mut impl ConnectionLike, path: impl Display) -> RedisResult<Value> {
    cmd("GET").arg(path.to_string()).query_async(rd).await
}

pub fn set_opt_ex<'a, K: ToRedisArgs, V: ToRedisArgs>(key: K, value: V, opt_exp: Option<usize>) -> Cmd {
    if let Some(exp) = opt_exp {
        Cmd::set_ex(key, value, exp)
    } else {
        Cmd::set(key, value)
    }
}

#[async_trait]
pub trait CacheWrite {
    type Stored: Send + FromRedisValue;

    fn write_cmd(&self, data: Self::Stored) -> Vec<Cmd>;
    
    async fn write(&self, rd: &mut (impl ConnectionLike + Send), data: Self::Stored) -> RedisResult<()> {
        execute_commands(rd, self.write_cmd(data)).await?;
        Ok(())
    }
}

#[async_trait]
pub trait CacheRead {
    type Stored: Send + FromRedisValue;

    fn read_cmd(&self) -> Vec<Cmd>;
    
    async fn read(&self, rd: &mut (impl ConnectionLike + Send)) -> RedisResult<Option<Self::Stored>> {
        let res: Value = execute_commands(rd, self.read_cmd()).await?;

        if res == Value::Nil {
            return Ok(None);
        } else {
            return Ok(Some(Self::Stored::from_redis_value(&res)?))
        }
    }
}

#[async_trait]
pub trait CacheInvalidate {
    fn invalidate_cmd(&self) -> Vec<Cmd>;
    
    async fn invalidate(&self, rd: &mut (impl ConnectionLike + Send)) -> RedisResult<()> {
        execute_commands(rd, self.invalidate_cmd()).await?;
        Ok(())
    }
}

pub async fn execute_commands(rd: &mut impl ConnectionLike, cmds: Vec<Cmd>) -> RedisResult<Value> {
    if cmds.len() == 1 {
        cmds[0].query_async(rd).await
    } else if cmds.len() > 1 {
        let mut pipe = Pipeline::new();
        let atomic_pipe = pipe.atomic();

        cmds.into_iter().for_each(|cmd| { atomic_pipe.add_command(cmd.clone()); });

        atomic_pipe.query_async(rd).await
    } else {
        Ok(Value::Nil)
    }
}

pub async fn pipeline_commands(pipe: &mut Pipeline, cmds: impl IntoIterator<Item = Cmd>) {
    cmds.into_iter().for_each(|cmd| { pipe.add_command(cmd); });
}
