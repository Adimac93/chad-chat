use redis::Value;
use redis::aio::ConnectionManager;
use redis::Client;
use redis::cmd;
use std::sync::Mutex;

const DEFAULT_BASE_REDIS_URL: &str = "redis://127.0.0.1:6379/";

static REDIS_DB_NUM: Mutex<i32> = Mutex::new(0);

pub async fn add_redis<'a, T>(fixtures: T) -> ConnectionManager
where
    T: IntoIterator<Item = &'a str> {
    dotenv::dotenv().ok();

    let base_redis_url = std::env::var("BASE_REDIS_URL").unwrap_or(DEFAULT_BASE_REDIS_URL.to_string());

    let db_num = {
        let mut val = REDIS_DB_NUM.lock().unwrap();
        *val += 1;
        *val
    };

    println!("Check database {db_num} for data");

    let client = Client::open(format!("{base_redis_url}{db_num}")).expect("Cannot establish redis connection");
    
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
