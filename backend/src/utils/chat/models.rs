use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use time::OffsetDateTime;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::socket::{GroupConnection, UserSender};

#[derive(Serialize, Deserialize)]
pub struct KickMessage {
    from: String,
    reason: String,
}

pub struct ChatState {
    groups: DashMap<Uuid, GroupTransmitter>,
}

impl ChatState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            groups: DashMap::new(),
        })
    }

    pub async fn add_user_connection(
        &self,
        group_id: Uuid,
        user_id: Uuid,
        sender: impl Into<UserSender> + Clone,
        connection_id: String,
    ) -> GroupConnection {
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
        group.conn.clone()
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

    pub async fn kick_user_from_group(&self, group_id: &Uuid, user_id: &Uuid) {
        if let Some(mut group) = self.groups.get_mut(group_id) {
            group.users.remove_all_user_connections(user_id).await;
        }
    }

    pub async fn change_user_connection(
        &self,
        user_id: &Uuid,
        group_id: &Uuid,
        new_group_id: &Uuid,
        connection_id: &str,
    ) -> Option<GroupConnection> {
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
    conn: GroupConnection,
    users: GroupChatState,
}

impl GroupTransmitter {
    // consider Arc tx and cloning
    fn new(user_id: Uuid, connection_id: String, sender: UserSender) -> Self {
        Self {
            conn: GroupConnection::new(100),
            users: GroupChatState::new(user_id, UserChatState::new(connection_id, sender)),
        }
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

    async fn remove_all_user_connections(&mut self, user_id: &Uuid) {
        let GroupChatState(group_users) = self;
        if let Some(mut user_senders) = group_users.lock().await.remove(user_id) {
            user_senders.remove_all_user_connections().await;
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

    async fn remove_all_user_connections(&mut self) -> Vec<UserSender> {
        let UserChatState(user_senders) = self;
        let mut senders = user_senders.lock().await;

        senders
            .drain()
            .map(|(id, sender)| sender)
            .collect::<Vec<UserSender>>()
    }

    async fn add_user_connection(&mut self, connection_id: String, sender: impl Into<UserSender>) {
        let UserChatState(user_senders) = self;
        user_senders
            .lock()
            .await
            .insert(connection_id, sender.into());
    }
}

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

#[derive(Serialize, Deserialize, Debug)]
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
