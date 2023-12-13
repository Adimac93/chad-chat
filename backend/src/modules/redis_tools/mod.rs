pub mod redis_path;

use std::fmt::Display;
use std::sync::Mutex;
use axum::async_trait;
use redis::{RedisResult, Pipeline};
use redis::aio::ConnectionLike;
use redis::{Client as RedisClient, cmd, Cmd, Value, aio::ConnectionManager};

const DEFAULT_BASE_REDIS_URL: &str = "redis://127.0.0.1:6379/";

static REDIS_DB_NUM: Mutex<i32> = Mutex::new(0);

fn parse_args(s: String) -> Vec<String> {
    s.split_whitespace().filter(|&x| x != "").map(|x| x.to_string()).collect()
}

pub async fn get_at(rd: &mut impl ConnectionLike, path: impl Display) -> RedisResult<Value> {
    cmd("GET").arg(path.to_string()).query_async(rd).await
}

pub trait RedisOps {
    type Stored: Send;

    fn write(&self, data: Self::Stored) -> Vec<Cmd>;
    fn read(&self) -> Vec<Cmd>;
    fn invalidate(&self) -> Vec<Cmd>;
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
