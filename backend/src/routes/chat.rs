use crate::models::{ChatState, Claims, GroupTransmitter};
use crate::utils::chat::*;
use crate::utils::groups::*;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Extension, Router,
};
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use time::{format_description, OffsetDateTime};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};
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

    // Message format
    let format = format_description::parse("[hour]:[minute]").unwrap();

    // Listen for user message
    while let Some(Ok(message)) = receiver.next().await {
        // Decode message
        let Ok(action) = ChatAction::try_from(message) else {
            error!("Invalid action");
            break;
        };
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
                let Ok(messages) = fetch_chat_messages(&pool,&group_id).await else {
                    error!("Cannot fetch group messages");
                    return;
                };

                for message in messages.iter() {
                    let Ok(login) = get_user_login_by_id(&pool, &message.user_id).await else {
                        error!("Failed to get user by login");
                        return;
                    };

                    let sent_at = message.sent_at.format(&format).unwrap();

                    if sender
                        .lock()
                        .await
                        .send(Message::Text(format!(
                            "{} {}: {}",
                            sent_at, login, message.content
                        )))
                        .await
                        .is_err()
                    {
                        error!("Failed to load messages");
                        break;
                    }
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
                    task.abort();
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
                if let Some(group_id) = current_group_id {
                    if let Some(tx) = ctx.clone() {
                        let payload = format!(
                            "{} {}: {}",
                            OffsetDateTime::now_utc().format(&format).unwrap(),
                            claims.login,
                            content
                        );
                        let res = tx.send(payload.clone());
                        debug!("Sent: {payload}");
                        debug!("Active transmitters: {res:?}");
                    }
                    let Ok(_) = create_message(&pool, &claims.id, &group_id, &content).await else {
                        return;
                    };
                } else {
                    error!("Cannot send message - group not selected");
                    return;
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
enum ChatAction {
    ChangeGroup { group_id: Uuid },
    SendMessage { content: String },
}

// {"ChangeGroup" : {"group_id": "asd-asdasd-asd-asd"}}
// {"SendMessage" : {"content": "Hello"}}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    msg_type: ChatAction,
}
impl TryFrom<Message> for ChatAction {
    type Error = ();

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Text(text) => {
                let action = serde_json::from_str::<ChatAction>(&text).map_err(|_e| ())?;
                Ok(action)
            },
            _ => Err(())
            // Message::Binary(_) => todo!(),
            // Message::Ping(_) => todo!(),
            // Message::Pong(_) => todo!(),
            // Message::Close(_) => todo!(),
        }
    }
}

async fn load_messages(pool: &PgPool, sender: &mut SplitSink<WebSocket, Message>, group_id: &Uuid) {
    let Ok(messages) = fetch_chat_messages(pool,group_id).await else {
        error!("Cannot fetch group messages");
        return;
    };

    for message in messages.iter() {
        if sender
            .send(Message::Text(format!("{}", message.content)))
            .await
            .is_err()
        {
            error!("Failed to load messages");
            break;
        }
    }
}
