use super::models::{AddresedMessage, GroupUserMessage, KickMessage};
use axum::extract::ws::{Message, WebSocket};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::broadcast::{self, Receiver};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace};
use uuid::Uuid;

pub struct Connection {
    pub user_conn: UserConnection,
    pub group_controller: Option<GroupController>,
}

pub struct GroupController {
    pub group_conn: GroupConnection,
    pub group_id: Uuid,
    receive_task: Option<JoinHandle<()>>,
}

impl GroupController {
    pub fn new(group_conn: GroupConnection, group_id: Uuid) -> Self {
        Self {
            group_conn,
            group_id,
            receive_task: None,
        }
    }
}

impl Connection {
    pub fn new(stream: WebSocket) -> Self {
        Self {
            user_conn: UserConnection::new(stream),
            group_controller: None,
        }
    }

    pub async fn listen_for_group_messages(&mut self) {
        if let Some(group_controller) = &mut self.group_controller {
            group_controller.receive_task = Some(
                self.user_conn
                    .sender
                    .listen(group_controller.group_conn.subscribe())
                    .await,
            )
        }
    }

    pub fn stop_listening_for_group_messages(&mut self) {
        if let Some(group_controller) = &mut self.group_controller {
            if let Some(receive_task) = &group_controller.receive_task {
                receive_task.abort();
                group_controller.receive_task = None;
            }
        }
    }
}
pub struct UserConnection {
    pub sender: UserSender,
    pub receiver: UserReceiver,
}

impl UserConnection {
    pub fn new(stream: WebSocket) -> Self {
        let (sender, receiver) = stream.split();
        Self {
            sender: UserSender::new(sender),
            receiver: UserReceiver::new(receiver),
        }
    }
}

#[derive(Clone)]
pub struct UserSender(Arc<Mutex<SplitSink<WebSocket, Message>>>);

impl UserSender {
    fn new(sender: SplitSink<WebSocket, Message>) -> Self {
        Self(Arc::new(Mutex::new(sender)))
    }

    /// Send server action to client
    pub async fn send(&self, action: &ServerAction) -> Result<(), axum::Error> {
        let UserSender(sender) = self;
        let msg = serde_json::to_string(action).unwrap();
        sender.lock().await.send(Message::Text(msg)).await
    }

    /// Listen to receiver messages and pass them to client
    pub async fn listen(&self, broadcast_receiver: GroupReceiver) -> JoinHandle<()> {
        let UserSender(sender) = self;
        let GroupReceiver(mut broadcast_receiver) = broadcast_receiver;

        let sender = sender.clone();
        tokio::spawn(async move {
            while let Ok(msg) = broadcast_receiver.recv().await {
                if sender.lock().await.send(Message::Text(msg)).await.is_err() {
                    error!("Error while sending message to the client");
                    break;
                }
            }
        })
    }
}

pub struct UserReceiver(SplitStream<WebSocket>);
impl UserReceiver {
    fn new(receiver: SplitStream<WebSocket>) -> Self {
        Self(receiver)
    }

    /// Get next client action
    pub async fn next_action(&mut self) -> ClientAction {
        let UserReceiver(receiver) = self;
        if let Some(conn) = receiver.next().await {
            return match conn {
                Ok(message) => ClientAction::new(message),
                Err(e) => {
                    debug!("Error while receiving message from stream {e}");
                    ClientAction::Forbidden
                }
            };
        }
        debug!("Data stream dropped");
        ClientAction::Close
    }
}

/// Server action send to client
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerAction {
    LoadMessages(Vec<GroupUserMessage>),
    LoadRequested(Vec<GroupUserMessage>),
    GroupInvite,
    Message(GroupUserMessage),
}

/// Client action send to server
#[derive(Serialize, Deserialize)]
pub enum ClientAction {
    ChangeGroup {
        group_id: Uuid,
    },
    SendMessage {
        content: String,
    },
    GroupInvite {
        group_id: Uuid,
    },
    RemoveUser {
        user_id: Uuid,
        group_id: Uuid,
        kick_message: KickMessage,
    },
    RequestMessages {
        loaded: i64,
    },
    Close,
    Forbidden,
}

impl ClientAction {
    fn new(message: Message) -> Self {
        match message {
            Message::Text(text) => {
                serde_json::from_str::<ClientAction>(&text).unwrap_or(ClientAction::Forbidden)
            }
            Message::Close(frame) => {
                match frame {
                    Some(frame) => {
                        trace!("Code: {} Reason: {}", frame.code, frame.reason);
                    }
                    None => {
                        trace!("Closed without frame")
                    }
                }
                debug!("Closing socket");
                ClientAction::Close
            }
            Message::Binary(_) => {
                info!("Binary message type is not supported");
                ClientAction::Forbidden
            }
            Message::Ping(_) => {
                info!("Ping message type is not supported");
                ClientAction::Forbidden
            }
            Message::Pong(_) => {
                info!("Pong message type is not supported");
                ClientAction::Forbidden
            }
        }
    }
}

pub struct GroupConnection {
    pub sender: GroupSender,
    pub receiver: GroupReceiver,
}

impl GroupConnection {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = broadcast::channel::<String>(capacity);
        Self {
            sender: GroupSender::new(sender),
            receiver: GroupReceiver::new(receiver),
        }
    }

    pub fn subscribe(&self) -> GroupReceiver {
        let GroupSender(sender) = &self.sender;
        GroupReceiver::new(sender.subscribe())
    }

    pub fn emit(&self) -> Self {
        let GroupSender(sender) = &self.sender;
        Self {
            sender: GroupSender::new(sender.clone()),
            receiver: GroupReceiver::new(sender.subscribe()),
        }
    }
}

impl Clone for GroupConnection {
    fn clone(&self) -> Self {
        let GroupSender(sender) = &self.sender;
        Self {
            sender: GroupSender::new(sender.clone()),
            receiver: GroupReceiver::new(sender.subscribe()),
        }
    }
}

pub struct GroupSender(broadcast::Sender<String>);

impl GroupSender {
    fn new(sender: broadcast::Sender<String>) -> Self {
        Self(sender)
    }

    /// Send server action to all group clients
    pub fn send(&self, action: &ServerAction) {
        let GroupSender(sender) = self;
        let msg = serde_json::to_string(action).unwrap();
        let res = sender.send(msg);
        match res {
            Ok(n) => {
                trace!("Action send to {n} group members");
            }
            Err(e) => {
                error!("Failed to execute server action for all active members");
            }
        }
    }
}

pub struct GroupReceiver(broadcast::Receiver<String>);

impl GroupReceiver {
    fn new(receiver: broadcast::Receiver<String>) -> Self {
        Self(receiver)
    }
}
