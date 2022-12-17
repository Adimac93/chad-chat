use axum::extract::ws::{WebSocket, Message};
use dashmap::DashMap;
use futures::stream::{SplitStream, SplitSink};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use time::OffsetDateTime;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

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
    pub fn new(user_id: Uuid, connection_id: String, sender: Arc<Mutex<SplitSink<WebSocket, Message>>>) -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            tx,
            users: UserConnections::new(user_id, Connections::new(connection_id, sender)),
        }
    }
}

pub struct Connections(HashMap<String, Arc<Mutex<SplitSink<WebSocket, Message>>>>);

impl Connections {
    pub fn new(connection_id: String, sender: Arc<Mutex<SplitSink<WebSocket, Message>>>) -> Self {
        Self(HashMap::from([(connection_id, sender)]))
    }

    pub fn remove(&mut self, connection_id: String) {
        let Connections(connections) = self;
        connections.remove(&connection_id);
    }
}

pub struct UserConnections(HashMap<Uuid, Connections>);

impl UserConnections {
    pub fn new(user_id: Uuid, connections: Connections) -> Self {
        Self(HashMap::from([(user_id, connections)]))
    }

    pub fn remove(&mut self, user_id: Uuid) {
        let UserConnections(connections) = self;
        connections.remove(&user_id);
    }

    pub fn get(&self, user_id: Uuid) -> Option<&Connections> {
        let UserConnections(connections) = self;
        connections.get(&user_id)
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
