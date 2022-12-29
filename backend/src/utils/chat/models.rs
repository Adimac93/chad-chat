use super::socket::{GroupChannel, GroupSender, ServerAction, UserChannel, UserSender};
use axum::extract::ws::WebSocket;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};
use std::{collections::HashMap, sync::Arc};
use time::OffsetDateTime;
use tokio::{sync::RwLock, task::JoinHandle};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct AddresedMessage {
    pub content: String,
    pub user_id: Uuid,
    pub group_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GroupUserMessageModel {
    pub nickname: String,
    pub content: String,
    pub sent_at: OffsetDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroupUserMessage {
    pub nickname: String,
    pub content: String,
    pub sat: i64,
}

impl GroupUserMessage {
    pub fn new(nickname: String, content: String) -> Self {
        Self {
            nickname,
            content,
            sat: OffsetDateTime::now_utc().unix_timestamp(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KickMessage {
    pub from: String,
    pub reason: String,
}
