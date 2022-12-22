use crate::utils::auth::models::Claims;
use crate::utils::chat::messages::fetch_last_messages_in_range;
use crate::utils::chat::models::*;
use crate::utils::chat::socket::{ClientAction, Connection, GroupController, ServerAction};
use crate::utils::chat::*;
use crate::utils::groups::*;
use axum::http::HeaderMap;
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Extension, Router,
};
use sqlx::PgPool;
use std::sync::Arc;
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
    let connection_id = get_connection_id(headers);
    ws.on_upgrade(|socket| chat_socket(socket, state, claims, pool, connection_id))
}

fn get_connection_id(headers: HeaderMap) -> String {
    if let Some(header) = headers.get("sec-websocket-key") {
        if let Ok(connection_id) = header.to_str() {
            return connection_id.to_string();
        }
    };
    error!("Failed to get sec-websocket-key");
    Uuid::new_v4().to_string()
}

pub async fn chat_socket(
    stream: WebSocket,
    state: Arc<ChatState>,
    claims: Claims,
    pool: PgPool,
    connection_id: String,
) {
    let mut conn = Connection::new(stream);

    // Listen for user message
    loop {
        // Decode message
        let action = conn.user_conn.receiver.next_action().await;
        // Interpret message
        match action {
            ClientAction::ChangeGroup { group_id } => {
                // Security checks
                if !connection_requirements(&pool, &group_id, &claims).await {
                    return;
                }

                // Abort listening to other group message transmitter
                conn.stop_listening_for_group_messages();

                let group_conn = if let Some(controller) = &conn.group_controller {
                    let Some(group_conn) = state.change_user_connection(
                                &claims.user_id,
                                &controller.group_id,
                                &group_id,
                                &connection_id,
                            )
                            .await else {
                                break;
                            };
                    group_conn
                } else {
                    state
                        .add_user_connection(
                            group_id,
                            claims.user_id,
                            conn.user_conn.sender.clone(),
                            connection_id.clone(),
                        )
                        .await
                };

                // Save current group controller
                conn.group_controller = Some(GroupController::new(group_conn, group_id));

                // Load messages
                let Ok(messages) = fetch_last_messages_in_range(&pool,&group_id,10,0).await else {
                    error!("ws closed: Cannot fetch group {} messages", &group_id);
                    return;
                };

                // Send messages json object
                let payload = ServerAction::LoadMessages(messages);

                if conn.user_conn.sender.send(&payload).await.is_err() {
                    error!("ws closed: Failed to load fetched messages");
                    return;
                }

                conn.listen_for_group_messages().await;
            }
            ClientAction::SendMessage { content } => {
                if content.len() > MAX_MESSAGE_LENGTH {
                    debug!(
                        "Message too long: the message length is {}, which is greater than {}",
                        content.len(),
                        MAX_MESSAGE_LENGTH
                    );
                    continue;
                }
                if let Some(controller) = &conn.group_controller {
                    let group_id = controller.group_id;
                    let nickname = get_group_nickname(&pool, &claims.user_id, &group_id)
                        .await
                        .unwrap_or("unknown_user".into());

                    let Ok(_) = create_message(&pool, &claims.user_id, &group_id, &content).await else {
                            error!("ws closed: Failed to save the message from the user {} ({}) in the database", &claims.user_id, &claims.login);
                            continue;
                        };

                    let payload = ServerAction::Message(GroupUserMessage::new(nickname, content));
                    debug!("Sent: {payload:#?}");

                    controller.group_conn.sender.send(&payload);
                } else {
                    debug!(
                        "Cannot send message from user {} ({}) - group not selected",
                        &claims.user_id, &claims.login
                    );
                    continue;
                }
            }
            ClientAction::RequestMessages { loaded } => {
                // Load messages
                if let Some(controller) = &conn.group_controller {
                    let group_id = controller.group_id;
                    info!("Requested messages");

                    let Ok(messages) = fetch_last_messages_in_range(&pool,&group_id,10,loaded).await else {
                        error!("ws closed: Cannot fetch group messages for user {} ({})", &claims.user_id, &claims.login);
                        return;
                    };

                    // Send messages json object
                    let payload = ServerAction::LoadRequested(messages);

                    if conn.user_conn.sender.send(&payload).await.is_err() {
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
            ClientAction::GroupInvite { group_id } => {
                let Ok(_is_member) = check_if_group_member(&pool, &claims.user_id, &group_id).await else {
                    error!("Failed to check whether a user {} ({}) is a group {} member (during sending a group invite)", &claims.user_id, &claims.login, &group_id);
                    return;
                };

                // TODO: send group invites in chat
            }
            ClientAction::RemoveUser {
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

                state.kick_user_from_group(&group_id, &user_id).await;
            }
            ClientAction::Close => {
                info!("WebSocket closed explicitly");
                return;
            }
            ClientAction::Forbidden => {
                info!("Action can't be handled");
                continue;
            }
        }
    }

    debug!("ws closed: User left the message loop");

    if let Some(controller) = &conn.group_controller {
        let group_id = controller.group_id;
        state
            .remove_user_connection(&group_id, &claims.user_id, &connection_id)
            .await;
    }
}

async fn connection_requirements(pool: &PgPool, group_id: &Uuid, claims: &Claims) -> bool {
    let Ok(is_group) = check_if_group_exists(pool,group_id).await else {
                    error!("ws closed: Cannot check if group {} exists", group_id);
                    return false;
                };
    if !is_group {
        info!("ws closed: Non existing group");
        return false;
    }
    let Ok(is_group_member) = check_if_group_member(pool,&claims.user_id,group_id).await else {
                    error!("ws closed: Cannot check if user {} ({}) is a group {} member", &claims.user_id, &claims.login, group_id);
                    return false;
                };
    if !is_group_member {
        info!(
            "ws closed: User {} ({}) isn't a group member",
            &claims.user_id, &claims.login
        );
        return false;
    }
    true
}
