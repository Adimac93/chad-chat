use backend::modules::redis_tools::add_redis;

#[tokio::test]
async fn redis_health_check() {
    let _rd = add_redis(1, vec!["", "PING", "SET a b", "SET c d", "SADD my_set 1 2 4 5"]).await;
}
