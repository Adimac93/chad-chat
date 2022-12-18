use axum::extract::ws::{CloseFrame, Message, WebSocket};
use dashmap::DashMap;
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, future::join_all,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use time::OffsetDateTime;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

use crate::utils::groups::models::GroupUser;

//type UserSender = Arc<Mutex<SplitSink<WebSocket, Message>>>;

pub struct UserSender(Arc<Mutex<SplitSink<WebSocket, Message>>>);

impl UserSender {
    pub async fn new(sender: impl Into<UserSender>) -> Self {
        sender.into()
    }

    async fn close(&self) {
        let UserSender(sender) = self;
        let res = sender
            .lock()
            .await
            .send(Message::Close(Some(CloseFrame {
                code: 1000,
                reason: "not".into(),
            })))
            .await;
    }
}

impl From<Arc<Mutex<SplitSink<WebSocket, Message>>>> for UserSender {
    fn from(value: Arc<Mutex<SplitSink<WebSocket, Message>>>) -> Self {
        Self(value)
    }
}

pub struct ChatState {
    pub groups: DashMap<Uuid, GroupTransmitter>,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            groups: DashMap::new(),
        }
    }

    pub fn add_group(&mut self, group_id: Uuid, group: GroupTransmitter) {
        self.groups.insert(group_id, group);
    }
}

pub struct GroupTransmitter {
    pub tx: broadcast::Sender<String>,
    pub users: GroupChatState,
}

impl GroupTransmitter {
    pub fn new(user_id: Uuid, connection_id: String, sender: UserSender) -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            tx,
            users: GroupChatState::new(user_id, UserChatState::new(connection_id, sender)),
        }
    }
}

pub struct GroupChatState(HashMap<Uuid, UserChatState>);

impl GroupChatState {
    pub fn new(user_id: Uuid, connections: UserChatState) -> Self {
        Self(HashMap::from([(user_id, connections)]))
    }

    pub fn remove_user(&mut self, user_id: Uuid) {
        let GroupChatState(group_users) = self;
        group_users.remove(&user_id);
    }

    pub fn remove_all_user_connections(&mut self, user_id: Uuid) {
        let GroupChatState(group_users) = self;
        group_users.entry(user_id).and_modify(|user_senders| {
            let UserChatState(user_senders) = user_senders;
            user_senders.clear();
        });
    }

    pub async fn remove_all_user_connections_with_a_frame(&mut self, user_id: Uuid) {
        let GroupChatState(group_users) = self;
        if let Some(user_senders) = group_users.get_mut(&user_id) {
            let UserChatState(user_senders_map) = user_senders;
            for key in user_senders_map.keys().cloned().collect::<Vec<String>>() {
                user_senders.remove_user_connection(key).await;
            }
        };
    }

    pub fn get_user(&self, user_id: Uuid) -> Option<&UserChatState> {
        let GroupChatState(group_users) = self;
        group_users.get(&user_id)
    }

    pub fn add_user_connection(&mut self, user_id: Uuid, sender: impl Into<UserSender> + Clone, connection_id: String) {
        let GroupChatState(group_users) = self;
        group_users
            .entry(user_id)
            .and_modify(|user_senders| user_senders.add_user_connection(connection_id.clone(), sender.clone()))
            .or_insert(UserChatState::new(connection_id, sender));
    }

    pub fn remove_user_connection(&mut self, user_id: Uuid, connection_id: String) {
        let GroupChatState(group_users) = self;
        group_users.entry(user_id).and_modify(|user_senders| {
            user_senders.remove_user_connection(connection_id);
        });
    }
}

pub struct UserChatState(HashMap<String, UserSender>);

impl UserChatState {
    pub fn new(connection_id: String, sender: impl Into<UserSender>) -> Self {
        Self(HashMap::from([(connection_id, sender.into())]))
    }

    async fn remove_user_connection(&mut self, connection_id: String) {
        let UserChatState(user_senders) = self;
        let Some(removed_connection) = user_senders.remove(&connection_id) else {
            return;
        };
        removed_connection.close().await;
    }

    fn add_user_connection(&mut self, connection_id: String, sender: impl Into<UserSender>) {
        let UserChatState(user_senders) = self;
        let res = user_senders.insert(connection_id, sender.into());
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserMessage {
    pub content: String,
    pub user_id: Uuid,
    pub group_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageModel {
    pub id: i32,
    pub content: String,
    pub user_id: Uuid,
    pub group_id: Uuid,
    pub sent_at: OffsetDateTime,
}
