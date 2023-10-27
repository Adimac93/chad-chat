use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use typeshare::typeshare;
use uuid::Uuid;

#[typeshare]
#[derive(Serialize, Deserialize, Debug)]
pub struct AddresedMessage {
    pub content: String,
    pub user_id: Uuid,
    pub group_id: Uuid,
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug)]
pub struct GroupUserMessageModel {
    pub nickname: String,
    pub content: String,
    pub sent_at: OffsetDateTime,
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroupUserMessage {
    pub nickname: String,
    pub content: String,
    pub sat: i32,
}

impl GroupUserMessage {
    pub fn new(nickname: String, content: String) -> Self {
        Self {
            nickname,
            content,
            sat: OffsetDateTime::now_utc().unix_timestamp() as i32,
        }
    }
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KickMessage {
    pub from: String,
    pub reason: String,
}
