use crate::models::{ChatState, Claims, GroupTransmitter};
use crate::utils::chat::messages::fetch_last_messages_in_range;
use crate::utils::chat::*;
use crate::utils::groups::*;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Extension, Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace};
use uuid::Uuid;

pub fn router() -> Router {
    Router::new()
        .route("/websocket", get(chat_handler))
        .layer(Extension(Arc::new(ChatState::new())))
}

async fn chat_handler(
    ws: WebSocketUpgrade,
    claims: Claims,
    Extension(state): Extension<Arc<ChatState>>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    ws.on_upgrade(|socket| chat_socket(socket, state, claims, pool))
}

pub async fn chat_socket(stream: WebSocket, state: Arc<ChatState>, claims: Claims, pool: PgPool) {
    // By splitting we can send and receive at the same time.
    let (sender, mut receiver) = stream.split();
    let sender = Arc::new(Mutex::new(sender));

    let mut ctx: Option<Sender<String>> = None;
    let mut recv_task: Option<JoinHandle<()>> = None;
    let mut current_group_id: Option<Uuid> = None;

    // Listen for user message
    while let Some(Ok(message)) = receiver.next().await {
        // Decode message

        
        let action = match ChatAction::try_from(message) {
            Ok(action) => action,
            Err(e) => {
                debug!("Invalid action {e}");
                break;
            },
        };

        // Interpret message
        match action {
            ChatAction::ChangeGroup { group_id } => {
                // Security checks
                let Ok(is_group) = check_if_group_exists(&pool,&group_id).await else {
                    error!("Cannot check if group exists");
                    return;
                };
                if !is_group {
                    info!("Non existing group");
                    return;
                }
                let Ok(is_group_member) = check_if_group_member(&pool,&claims.id,&group_id).await else {
                    error!("Cannot check if user is a group member");
                    return;
                };
                if !is_group_member {
                    info!("User isn't a group member");
                    return;
                }
                // Save currend group id
                current_group_id = Some(group_id);

                // Load messages
                let Ok(messages) = fetch_last_messages_in_range(&pool,&group_id,10,0).await else {
                    error!("Cannot fetch group messages");
                    return;
                };

                let mut payload_messages = vec![];
                for message in messages.into_iter() {
                    let Ok(login) = get_user_login_by_id(&pool, &message.user_id).await else {
                        // ?User deleted account
                        error!("Failed to get user by login");
                        return;
                    };

                    payload_messages.push(UserMessage {
                        sender: login,
                        content: message.content,
                        sat: message.sent_at.unix_timestamp(),
                    })
                }

                // Send messages json object
                let payload = SocketMessage::LoadMessages(payload_messages);
                let msg = serde_json::to_string(&payload).unwrap();

                if sender.lock().await.send(Message::Text(msg)).await.is_err() {
                    error!("Failed to load messages");
                    break;
                }

                // Fetch group transmitter or create one & add user as online member of group
                let group = state
                    .groups
                    .entry(group_id)
                    .and_modify(|group_tx| {
                        group_tx.users.insert(claims.id);
                    })
                    .or_insert(GroupTransmitter::new());

                // Group channels
                let tx = group.tx.clone();
                let mut rx = tx.subscribe();
                ctx = Some(tx);

                // Send message to cliend side
                if let Some(task) = recv_task {
                    // Abort listening to other group message transmitter
                    task.abort()
                };
                let sender_cloned = sender.clone();
                recv_task = Some(tokio::spawn(async move {
                    while let Ok(msg) = rx.recv().await {
                        if sender_cloned
                            .lock()
                            .await
                            .send(Message::Text(msg))
                            .await
                            .is_err()
                        {
                            debug!("Error while seding message to client");
                            break;
                        }
                    }
                }));
            }
            ChatAction::SendMessage { content } => {
                if content.len() > 2000 {
                    debug!("Message too long");
                    return;
                }
                if let Some(group_id) = current_group_id {
                    if let Some(tx) = ctx.clone() {
                        let payload = SocketMessage::Message(UserMessage {
                            content: content.to_string(),
                            sat: OffsetDateTime::now_utc().unix_timestamp(),
                            sender: claims.login.to_string(),
                        });
                        debug!("Sent: {payload:#?}");
                        let msg = serde_json::to_string(&payload).unwrap();
                        let res = tx.send(msg);
                        match res {
                            Ok(count) => {
                                debug!("Active transmitters: {count}");
                            },
                            Err(e) => {
                                error!("{e}")
                            },
                        }
                    }
                    let Ok(_) = create_message(&pool, &claims.id, &group_id, &content).await else {
                        return;
                    };
                } else {
                    error!("Cannot send message - group not selected");
                    return;
                }
            }
            ChatAction::RequestMessages { loaded } => {
                // Load messages
                if let Some(group_id) = current_group_id {
                    info!("Requested messages");
                    let Ok(messages) = fetch_last_messages_in_range(&pool,&group_id,10,loaded).await else {
                        
                        error!("Cannot fetch group messages");
                        return;
                    };
                    

                    let mut payload_messages = vec![];
                    for message in messages.into_iter() {
                        let Ok(login) = get_user_login_by_id(&pool, &message.user_id).await else {
                        // ?User deleted account
                        error!("Failed to get user by login");
                        return;
                    };

                        payload_messages.push(UserMessage {
                            sender: login,
                            content: message.content,
                            sat: message.sent_at.unix_timestamp(),
                        })
                    }
                    trace!("{payload_messages:#?}");
                    // Send messages json object
                    let payload = SocketMessage::LoadRequested(payload_messages);
                    let msg = serde_json::to_string(&payload).unwrap();

                    if sender.lock().await.send(Message::Text(msg)).await.is_err() {
                        error!("Failed to load messages");
                        break;
                    }
                } else {
                    error!("Cannot fetch requested messages - group not selected");
                    return;
                }
            }
            ChatAction::GroupInvite { group_id } => {
                let Ok(is_member) = check_if_group_member(&pool, &claims.id, &group_id).await else {
                    return;
                };
                if is_member {}
            }
            ChatAction::Close => {
                return;
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
enum ChatAction {
    ChangeGroup { group_id: Uuid },
    SendMessage { content: String },
    GroupInvite { group_id: Uuid },
    RequestMessages { loaded: i64 },
    Close
}

// {"ChangeGroup" : {"group_id": "asd-asdasd-asd-asd"}}
// {"SendMessage" : {"content": "Hello"}} -> {"message": {"content": "Hello", "time": 1669233892}}
// {"GroupInvite": {"group_id": "asd-asdasd-asd-asd"}} -> {"invite": {"group_id": "asd-asdasd-asd-asd"}}

impl TryFrom<Message> for ChatAction {
    type Error = String;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Text(text) => {
                let action = serde_json::from_str::<ChatAction>(&text).map_err(|e| e.to_string())?;
                Ok(action)
            },
           
            Message::Binary(_) => Err(format!("Binary")),
            Message::Ping(_) => Err(format!("Ping")),
            Message::Pong(_) => Err(format!("Pong")),
            Message::Close(_frame) => {
                debug!("Closing socket");
                Ok(ChatAction::Close)
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum SocketMessage {
    LoadMessages(Vec<UserMessage>),
    LoadRequested(Vec<UserMessage>),
    GroupInvite,
    Message(UserMessage),
}

#[derive(Serialize, Deserialize, Debug)]
struct UserMessage {
    sender: String,
    sat: i64,
    content: String,
}
