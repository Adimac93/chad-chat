pub mod redis_path;

use std::fmt::Display;
use redis::RedisResult;
use redis::{Client as RedisClient, cmd, Value, aio::ConnectionManager};

const DEFAULT_BASE_REDIS_URL: &str = "redis://127.0.0.1:6379/";

pub async fn add_redis<T>(db_number: i32, fixtures: T) -> ConnectionManager
where
    T: IntoIterator,
    <T as IntoIterator>::Item: Into<String> {
    dotenv::dotenv().ok();

    let base_redis_url = std::env::var("BASE_REDIS_URL").unwrap_or(DEFAULT_BASE_REDIS_URL.to_string());

    let client =
        RedisClient::open(format!("{base_redis_url}{db_number}")).expect("Cannot establish redis connection");
    
    let mut rd = client
        .get_tokio_connection_manager()
        .await
        .expect("Failed to get redis connection manager");

    rd.send_packed_command(&cmd("FLUSHDB")).await.unwrap();

    for x in fixtures {
        let s: String = x.into();
        if s.trim().is_empty() {
            continue;
        };
        let args = parse_args(s);
        let _: Value = cmd(&args[0]).arg(&args[1..]).query_async(&mut rd).await.unwrap();
    };

    rd
}

fn parse_args(s: String) -> Vec<String> {
    s.split_whitespace().filter(|&x| x != "").map(|x| x.to_string()).collect()
}

pub async fn get_at(rd: &mut ConnectionManager, path: impl Display) -> RedisResult<Value> {
    cmd("GET").arg(path.to_string()).query_async(rd).await
}
