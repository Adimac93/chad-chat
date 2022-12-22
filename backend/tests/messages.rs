use uuid::Uuid;
mod tools;
use backend::utils::chat::{messages::fetch_last_messages_in_range, models::GroupUserMessage};
use sqlx::PgPool;

#[sqlx::test(fixtures("users", "groups", "messages"))]
async fn partial(pool: PgPool) {
    let group_id = Uuid::try_from("b8c9a317-a456-458f-af88-01d99633f8e2").unwrap();

    let mut loaded_messages = 0;
    let mut buffer: Vec<GroupUserMessage> = Vec::new();

    let load_on_fetch = 2;
    let loadings = 3;
    let expected = load_on_fetch * loadings;
    for _ in 0..loadings {
        let messages =
            fetch_last_messages_in_range(&pool, &group_id, load_on_fetch, loaded_messages)
                .await
                .unwrap();
        loaded_messages += messages.len() as i64;
        buffer.extend(messages);
    }

    assert_eq!(loaded_messages, expected);
    assert_eq!(buffer.len() as i64, expected);
}
