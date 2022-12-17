use axum::extract::ws::{CloseFrame, Message, WebSocket};
use dashmap::DashMap;
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use time::OffsetDateTime;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

//type UserSender = Arc<Mutex<SplitSink<WebSocket, Message>>>;

struct UserSender(Mutex<SplitSink<WebSocket, Message>>);

impl UserSender {
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
impl From<Mutex<SplitSink<WebSocket, Message>>> for UserSender {
    fn from(value: Mutex<SplitSink<WebSocket, Message>>) -> Self {
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
}

pub struct GroupTransmitter {
    pub tx: broadcast::Sender<String>,
    pub users: UserConnections,
}

impl GroupTransmitter {
    pub fn new(user_id: Uuid, connection_id: String, sender: UserSender) -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            tx,
            users: UserConnections::new(user_id, Connections::new(connection_id, sender)),
        }
    }
}

pub struct Connections(HashMap<String, UserSender>);

impl Connections {
    pub fn new(connection_id: String, sender: impl Into<UserSender>) -> Self {
        Self(HashMap::from([(connection_id, sender.into())]))
    }

    async fn remove(&mut self, connection_id: String) {
        let Connections(connections) = self;
        let Some(connection) = connections.remove(&connection_id) else {
            return;
        };
        connection.close().await;
    }

    fn add(&mut self, connection_id: String, sender: impl Into<UserSender>) {
        let Connections(connections) = self;
        let res = connections.insert(connection_id, sender.into());
    }
}

pub struct UserConnections(HashMap<Uuid, Connections>);

impl UserConnections {
    pub fn new(user_id: Uuid, connections: Connections) -> Self {
        Self(HashMap::from([(user_id, connections)]))
    }

    pub fn remove_all(&mut self, user_id: Uuid) {
        let UserConnections(connections) = self;
        connections.remove(&user_id);
    }

    // how to confuse people with variable names
    pub fn remove(&mut self, user_id: Uuid, connection_id: String) {
        let UserConnections(connections) = self;
        connections.entry(user_id).and_modify(|connections| {
            let Connections(connections) = connections;
            connections.clear();
        });
    }

    pub fn get(&self, user_id: Uuid) -> Option<&Connections> {
        let UserConnections(connections) = self;
        connections.get(&user_id)
    }

    pub fn add(&mut self, user_id: Uuid, sender: impl Into<UserSender>, connection_id: String) {
        let UserConnections(user_connections) = self;
        user_connections
            .entry(user_id)
            .and_modify(|connection| connection.add(connection_id, sender))
            .or_insert(Connections::new(connection_id, sender));
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
