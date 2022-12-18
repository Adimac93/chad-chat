use axum::extract::ws::{CloseFrame, Message, WebSocket};
use dashmap::DashMap;
use futures::{
    stream::SplitSink,
    SinkExt,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use time::OffsetDateTime;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

//type UserSender = Arc<Mutex<SplitSink<WebSocket, Message>>>;

pub struct UserSender(Arc<Mutex<SplitSink<WebSocket, Message>>>);

impl UserSender {
    pub async fn new(sender: impl Into<UserSender>) -> Self {
        sender.into()
    }

    async fn close(&self) {
        let UserSender(sender) = self;
        let _res = sender
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

    // fn add_group(&mut self, group_id: Uuid, user_id: Uuid, connection_id: String, sender: UserSender) {
    //     self.groups.insert(group_id, GroupTransmitter::new(user_id, connection_id, sender));
    // }

    pub fn get_target_group(&self, group_id: Uuid) -> Option<GroupChatState> {
        let group_ref = self.groups.get(&group_id);
        let GroupChatState(res) = &match group_ref.as_deref() {
            Some(group_tx) => group_tx,
            None => return None,
        }
        .users;

        Some(GroupChatState(Arc::clone(res)))
    }

    // async fn get_target_user(&self, group_id: Uuid, user_id: Uuid) -> Option<UserChatState> {
    //     let Some(GroupChatState(group)) = self.get_target_group(group_id) else {
    //         return None;
    //     };

    //     let res = match group.lock().await.get(&user_id) {
    //         Some(UserChatState(user)) => Some(UserChatState(Arc::clone(&user))),
    //         None => None,
    //     };
        
    //     res
    // }
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

pub struct GroupChatState(Arc<Mutex<HashMap<Uuid, UserChatState>>>);

impl GroupChatState {
    fn new(user_id: Uuid, connections: UserChatState) -> Self {
        Self(Arc::new(Mutex::new(HashMap::from([(user_id, connections)]))))
    }

    // fn remove_user(&mut self, user_id: Uuid) {
    //     let GroupChatState(group_users) = self;
    //     group_users.remove(&user_id);
    // }

    // async fn remove_all_user_connections(&mut self, user_id: Uuid) {
    //     let GroupChatState(group_users) = self;
    //     if let Some(user_senders) = group_users.get_mut(&user_id) {
    //         let UserChatState(user_senders) = user_senders;
    //         user_senders.lock().await.clear();
    //     }
    // }

    pub async fn remove_all_user_connections_with_a_frame(&mut self, user_id: Uuid) {
        let GroupChatState(group_users) = self;
        let mut data = group_users.lock().await;
        if let Some(user_senders) = data.get_mut(&user_id) {
            let UserChatState(user_senders_map) = user_senders;
            let vec = {
                user_senders_map.lock().await.keys().cloned().collect::<Vec<String>>()
            };
            for key in vec {
                user_senders.remove_user_connection(key).await;
            }

            data.remove(&user_id);
        };
    }

    // async fn get_user(&self, user_id: Uuid) -> Option<UserChatState> {
    //     let GroupChatState(group_users) = self;
    //     match group_users.lock().await.get(&user_id) {
    //         Some(UserChatState(s)) => Some(UserChatState(Arc::clone(s))),
    //         None => None,
    //     }
    // }

    pub async fn add_user_connection(&mut self, user_id: Uuid, sender: impl Into<UserSender> + Clone, connection_id: String) {
        let GroupChatState(group_users) = self;
        let mut data = group_users.lock().await;
        match data.get_mut(&user_id) {
            Some(user_senders) => user_senders.add_user_connection(connection_id.clone(), sender.clone()).await,
            None => { let _ = data.insert(user_id, UserChatState::new(connection_id, sender)); },
        }
    }

    pub async fn remove_user_connection(&mut self, user_id: Uuid, connection_id: String) {
        let GroupChatState(group_users) = self;
        let mut data = group_users.lock().await;
        if let Some(user_senders) = data.get_mut(&user_id) {
            user_senders.remove_user_connection(connection_id).await;
        }
    }
}

pub struct UserChatState(Arc<Mutex<HashMap<String, UserSender>>>);

impl UserChatState {
    fn new(connection_id: String, sender: impl Into<UserSender>) -> Self {
        Self(Arc::new(Mutex::new(HashMap::from([(connection_id, sender.into())]))))
    }

    async fn remove_user_connection(&mut self, connection_id: String) {
        let UserChatState(user_senders) = self;
        let Some(removed_connection) = user_senders.lock().await.remove(&connection_id) else {
            return;
        };
        removed_connection.close().await;
    }

    async fn add_user_connection(&mut self, connection_id: String, sender: impl Into<UserSender>) {
        let UserChatState(user_senders) = self;
        let _res = user_senders.lock().await.insert(connection_id, sender.into());
    }
    
    pub async fn remove_all_user_connections_with_a_frame(&mut self) {
        let UserChatState(user_senders_map) = self;
        let vec = {
            user_senders_map.lock().await.keys().cloned().collect::<Vec<String>>()
        };
        for key in vec {
            self.remove_user_connection(key).await;
        }
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
