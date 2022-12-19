use crate::utils::auth::models::Claims;
use crate::utils::chat::messages::fetch_last_messages_in_range;
use crate::utils::chat::models::*;
use crate::utils::chat::*;
use crate::utils::groups::*;

use axum::http::HeaderMap;
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

const MAX_MESSAGE_LENGTH: usize = 2000;

pub fn router() -> Router {
    Router::new()
        .route("/websocket", get(chat_handler))
        .layer(Extension(Arc::new(ChatState::new())))
}

async fn chat_handler(
    // can't get value TypedHeader(key): TypedHeader<SecWebsocketKey>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    claims: Claims,
    Extension(state): Extension<Arc<ChatState>>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    ws.on_upgrade(|socket| chat_socket(socket, state, claims, pool, headers))
}

pub async fn chat_socket(
    stream: WebSocket,
    state: Arc<ChatState>,
    claims: Claims,
    pool: PgPool,
    headers: HeaderMap,
) {
    // let connection_id = Uuid::new_v4();
    let Some(key_header) = headers.get("sec-websocket-key") else {
        return;
    };
    let Ok(connection_id) = key_header.to_str() else {
        return;
    };
    let connection_id = connection_id.to_string();

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
                debug!("ws closed: Invalid action {e}");
                return;
            }
        };

        // Interpret message
        match action {
            ChatAction::ChangeGroup { group_id } => {
                // Security checks
                let Ok(is_group) = check_if_group_exists(&pool,&group_id).await else {
                    error!("ws closed: Cannot check if group {} exists", &group_id);
                    return;
                };
                if !is_group {
                    info!("ws closed: Non existing group");
                    return;
                }
                let Ok(is_group_member) = check_if_group_member(&pool,&claims.user_id,&group_id).await else {
                    error!("ws closed: Cannot check if user {} ({}) is a group {} member", &claims.user_id, &claims.login, &group_id);
                    return;
                };
                if !is_group_member {
                    info!(
                        "ws closed: User {} ({}) isn't a group member",
                        &claims.user_id, &claims.login
                    );
                    return;
                }
                
                // Abort listening to other group message transmitter
                if let Some(task) = recv_task {  
                    task.abort()
                };

                // Remove user from the earlier connection and fetch new 
                let (tx, mut rx) = match current_group_id {
                    Some(previous_group_id) => {
                        let Some(conn) = state
                            .change_user_connection(
                                &claims.user_id,
                                &previous_group_id,
                                &group_id,
                                &connection_id,
                            )
                            .await
                        else {
                           break; 
                        };
                        conn
                    },
                    None => {
                        state.add_user_connection(group_id, claims.user_id, sender.clone(), connection_id.clone()).await
                    }
                };

                // Save current connection 
                ctx = Some(tx);
                current_group_id = Some(group_id);

                // Load messages
                let Ok(messages) = fetch_last_messages_in_range(&pool,&group_id,10,0).await else {
                    error!("ws closed: Cannot fetch group {} messages", &group_id);
                    return;
                };

                let mut payload_messages = vec![];
                for message in messages.into_iter() {
                    let Ok(nickname) = get_group_nickname(&pool, &message.user_id,&group_id).await else {
                        // ?User deleted account
                        error!("ws closed: Failed to get user by id: {}", &message.user_id);
                        return;
                    };

                    payload_messages.push(UserMessage {
                        sender: nickname,
                        content: message.content,
                        sat: message.sent_at.unix_timestamp(),
                    })
                }

                // Send messages json object
                let payload = SocketMessage::LoadMessages(payload_messages);
                let msg = serde_json::to_string(&payload).unwrap();

                if sender.lock().await.send(Message::Text(msg)).await.is_err() {
                    error!("ws closed: Failed to load fetched messages");
                    return;
                }
                
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
                            error!("Error while sending message to the client");
                            break;
                        }
                    }
                }));
            }
            ChatAction::SendMessage { content } => {
                if content.len() > MAX_MESSAGE_LENGTH {
                    debug!(
                        "Message too long: the message length is {}, which is greater than {}",
                        content.len(),
                        MAX_MESSAGE_LENGTH
                    );
                    continue;
                }
                if let Some(group_id) = current_group_id {
                    if let Some(tx) = ctx.clone() {
                        let Ok(nickname) = get_group_nickname(&pool, &claims.user_id,&group_id).await else {
                            // ?User deleted account
                            error!("ws closed: Failed to get user by id: {}", &claims.user_id);
                            return;
                        };
                        let payload = SocketMessage::Message(UserMessage {
                            content: content.to_string(),
                            sat: OffsetDateTime::now_utc().unix_timestamp(),
                            sender: nickname,
                        });
                        debug!("Sent: {payload:#?}");
                        let Ok(msg) = serde_json::to_string(&payload) else {
                            error!("ws closed: Failed to convert a message to its json form");
                            return;
                        };
                        let res = tx.send(msg);
                        match res {
                            Ok(count) => {
                                debug!("Active transmitters: {count}");
                            }
                            Err(e) => {
                                error!("{e}")
                            }
                        }
                    }
                    let Ok(_) = create_message(&pool, &claims.user_id, &group_id, &content).await else {
                        error!("ws closed: Failed to save the message from the user {} ({}) in the database", &claims.user_id, &claims.login);
                        return;
                    };
                } else {
                    debug!(
                        "Cannot send message from user {} ({}) - group not selected",
                        &claims.user_id, &claims.login
                    );
                    continue;
                }
            }
            ChatAction::RequestMessages { loaded } => {
                // Load messages
                if let Some(group_id) = current_group_id {
                    info!("Requested messages");
                    let Ok(messages) = fetch_last_messages_in_range(&pool,&group_id,10,loaded).await else {
                        error!("ws closed: Cannot fetch group messages for user {} ({})", &claims.user_id, &claims.login);
                        return;
                    };

                    let mut payload_messages = vec![];
                    for message in messages.into_iter() {
                        let Ok(nickname) = get_group_nickname(&pool, &message.user_id,&group_id).await else {
                        // ?User deleted account
                        error!("ws closed: Failed to get nickname of the user {} ({}) and the group {}", &message.user_id, &claims.login, &group_id);
                        return;
                    };
                        payload_messages.push(UserMessage {
                            sender: nickname,
                            content: message.content,
                            sat: message.sent_at.unix_timestamp(),
                        })
                    }

                    trace!("{payload_messages:#?}");
                    // Send messages json object
                    let payload = SocketMessage::LoadRequested(payload_messages);
                    let Ok(msg) = serde_json::to_string(&payload) else {
                        error!("ws closed: Failed to convert a message to its json form");
                        return;
                    };

                    if sender.lock().await.send(Message::Text(msg)).await.is_err() {
                        error!(
                            "Failed to load messages for user {} ({})",
                            &claims.user_id, &claims.login
                        );
                        return;
                    }
                } else {
                    debug!("Cannot fetch requested messages - group not selected");
                    continue;
                }
            }
            ChatAction::GroupInvite { group_id } => {
                let Ok(_is_member) = check_if_group_member(&pool, &claims.user_id, &group_id).await else {
                    error!("Failed to check whether a user {} ({}) is a group {} member (during sending a group invite)", &claims.user_id, &claims.login, &group_id);
                    return;
                };

                // TODO: a feature to send group invites in chat
            }
            ChatAction::RemoveUser {
                user_id,
                group_id,
                kick_message,
            } => {
                match check_if_group_member(&pool, &user_id, &group_id).await {
                    Ok(false) => {
                        debug!(
                            "Cannot remove user {} from group {} - user is not a group member",
                            &user_id, &group_id
                        );
                        continue;
                    }
                    Err(_) => {
                        error!("ws closed: Failed to check whether a user {} is a group {} member (during user removal)", &user_id, &group_id);
                        return;
                    }
                    _ => (),
                };

                let Ok(_) = try_remove_user_from_group(&pool, user_id, group_id).await else {
                    error!("ws closed: Failed to remove user {} from a group {}", &user_id, &group_id);
                    return;
                };

                state
                    .remove_all_user_connections(&group_id, &user_id, &kick_message)
                    .await;
            }
            ChatAction::Close => {
                info!("WebSocket closed explicitly");
                return;
            }
        }
    }

    debug!("ws closed: User left the message loop");

    if let Some(previous_group_id) = current_group_id {
        state
            .remove_user_connection(&previous_group_id, &claims.user_id, &connection_id)
            .await;
    }
}

#[derive(Serialize, Deserialize)]
enum ChatAction {
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
}

// {"ChangeGroup" : {"group_id": "asd-asdasd-asd-asd"}}
// {"SendMessage" : {"content": "Hello"}} -> {"message": {"content": "Hello", "time": 1669233892}}
// {"GroupInvite": {"group_id": "asd-asdasd-asd-asd"}} -> {"invite": {"group_id": "asd-asdasd-asd-asd"}}

impl TryFrom<Message> for ChatAction {
    type Error = String;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Text(text) => {
                let action =
                    serde_json::from_str::<ChatAction>(&text).map_err(|e| e.to_string())?;
                Ok(action)
            }

            Message::Binary(_) => Err(format!("Binary message type is not supported")),
            Message::Ping(_) => Err(format!("Ping message type is not supported")),
            Message::Pong(_) => Err(format!("Pong message type is not supported")),
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
                Ok(ChatAction::Close)
            }
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
