use axum::extract::ws::{CloseFrame, Message, WebSocket};
use dashmap::DashMap;
use futures::{stream::SplitSink, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use time::OffsetDateTime;
use tokio::sync::{
    broadcast::{self, Receiver, Sender},
    Mutex,
};
use tracing::error;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct KickMessage {
    from: String,
    reason: String,
}
#[derive(Clone)]
pub struct UserSender(Arc<Mutex<SplitSink<WebSocket, Message>>>);

impl UserSender {
    async fn kick(self, kick_message: &KickMessage) {
        let UserSender(sender) = self;
        let Ok(msg) = serde_json::to_string(kick_message) else {
            error!("Failed to parse kick message");
            return;
        };
        let res = sender.lock().await.send(Message::Text(msg)).await;
    }
}

impl From<Arc<Mutex<SplitSink<WebSocket, Message>>>> for UserSender {
    fn from(value: Arc<Mutex<SplitSink<WebSocket, Message>>>) -> Self {
        Self(value)
    }
}

pub struct ChatState {
    groups: DashMap<Uuid, GroupTransmitter>,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            groups: DashMap::new(),
        }
    }

    pub async fn add_user_connection(
        &self,
        group_id: Uuid,
        user_id: Uuid,
        sender: impl Into<UserSender> + Clone,
        connection_id: String,
    ) -> (Sender<String>, Receiver<String>) {
        let group = match self.groups.get_mut(&group_id) {
            Some(mut group) => {
                group
                    .users
                    .add_user_connection(user_id, sender, connection_id.clone())
                    .await;
                group
            }
            None => self.groups.entry(group_id).or_insert(GroupTransmitter::new(
                user_id,
                connection_id.clone(),
                sender.into(),
            )),
        };
        group.emit()
    }

    pub async fn remove_user_connection(
        &self,
        group_id: &Uuid,
        user_id: &Uuid,
        connection_id: &str,
    ) -> Option<UserSender> {
        if let Some(mut group) = self.groups.get_mut(group_id) {
            return group
                .users
                .remove_user_connection(user_id, connection_id)
                .await;
        }
        None
    }

    pub async fn remove_all_user_connections(
        &self,
        group_id: &Uuid,
        user_id: &Uuid,
        kick_message: &KickMessage,
    ) {
        if let Some(mut group) = self.groups.get_mut(group_id) {
            group
                .users
                .remove_all_user_connections(user_id, kick_message)
                .await;
        }
    }

    pub async fn change_user_connection(
        &self,
        user_id: &Uuid,
        group_id: &Uuid,
        new_group_id: &Uuid,
        connection_id: &str,
    ) -> Option<(Sender<String>, Receiver<String>)> {
        if let Some(user_sender) = self
            .remove_user_connection(group_id, user_id, connection_id)
            .await
        {
            return Some(
                self.add_user_connection(
                    new_group_id.clone(),
                    user_id.clone(),
                    user_sender,
                    connection_id.to_string(),
                )
                .await,
            );
        }
        None
    }
}

pub struct GroupTransmitter {
    tx: broadcast::Sender<String>,
    users: GroupChatState,
}

impl GroupTransmitter {
    // consider Arc tx and cloning
    fn new(user_id: Uuid, connection_id: String, sender: UserSender) -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            tx,
            users: GroupChatState::new(user_id, UserChatState::new(connection_id, sender)),
        }
    }

    fn emit(&self) -> (Sender<String>, Receiver<String>) {
        (self.tx.clone(), self.tx.subscribe())
    }
}
/// Arc<DashMap<Uuid (group id), Group
pub struct GroupChatState(Arc<Mutex<HashMap<Uuid, UserChatState>>>);

impl GroupChatState {
    fn new(user_id: Uuid, connections: UserChatState) -> Self {
        Self(Arc::new(Mutex::new(HashMap::from([(
            user_id,
            connections,
        )]))))
    }

    async fn remove_user_connection(
        &mut self,
        user_id: &Uuid,
        connection_id: &str,
    ) -> Option<UserSender> {
        let GroupChatState(group_users) = self;
        if let Some(user_senders) = group_users.lock().await.get_mut(user_id) {
            return user_senders.remove_user_connection(connection_id).await;
        }
        None
    }

    async fn remove_all_user_connections(&mut self, user_id: &Uuid, kick_message: &KickMessage) {
        let GroupChatState(group_users) = self;
        if let Some(mut user_senders) = group_users.lock().await.remove(user_id) {
            user_senders.remove_all_user_connections(kick_message).await;
        };
    }

    async fn add_user_connection(
        &mut self,
        user_id: Uuid,
        sender: impl Into<UserSender> + Clone,
        connection_id: String,
    ) {
        let GroupChatState(group_users) = self;
        let mut data = group_users.lock().await;
        match data.get_mut(&user_id) {
            Some(user_senders) => {
                user_senders
                    .add_user_connection(connection_id, sender)
                    .await
            }
            None => {
                let _ = data.insert(user_id, UserChatState::new(connection_id, sender));
            }
        }
    }
}

pub struct UserChatState(Arc<Mutex<HashMap<String, UserSender>>>);

impl UserChatState {
    fn new(connection_id: String, sender: impl Into<UserSender>) -> Self {
        Self(Arc::new(Mutex::new(HashMap::from([(
            connection_id,
            sender.into(),
        )]))))
    }

    async fn remove_user_connection(&mut self, connection_id: &str) -> Option<UserSender> {
        let UserChatState(user_senders) = self;
        user_senders.lock().await.remove(connection_id)
    }

    async fn remove_all_user_connections(&mut self, kick_message: &KickMessage) {
        let UserChatState(user_senders) = self;
        let mut senders = user_senders.lock().await;

        for (_, sender) in senders.drain() {
            sender.kick(kick_message).await;
        }
    }

    async fn add_user_connection(&mut self, connection_id: String, sender: impl Into<UserSender>) {
        let UserChatState(user_senders) = self;
        user_senders
            .lock()
            .await
            .insert(connection_id, sender.into());
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
