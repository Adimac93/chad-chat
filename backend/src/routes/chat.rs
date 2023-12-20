use crate::state::{AppState, RdPool};
use crate::utils::auth::models::Claims;
use crate::utils::chat::messages::fetch_last_messages_in_range;
use crate::utils::chat::models::*;
use crate::utils::chat::socket::{ChatState, ClientAction, ServerAction, UserController};
use crate::utils::chat::*;
use crate::utils::groups::*;
use crate::utils::roles::models::{Gate, Role};
use crate::utils::roles::privileges::Privilege;
use crate::utils::roles::{
    get_user_role, get_all_privileges, set_privileges, set_role,
};
use axum::extract::State;
use axum::http::HeaderMap;
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use sqlx::PgPool;
use tracing::{debug, error, info};
use uuid::Uuid;

const MAX_MESSAGE_LENGTH: usize = 2000;

pub fn router() -> Router<AppState> {
    Router::new().route("/websocket", get(chat_handler))
}

async fn chat_handler(
    // can't get value TypedHeader(key): TypedHeader<SecWebsocketKey>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    claims: Claims,
    State(state): State<ChatState>,
    State(pg): State<PgPool>,
    State(rd): State<RdPool>,
    State(gate): State<Gate<Role, (Uuid, Uuid)>>,
) -> Response {
    let connection_id = get_connection_id(headers);
    ws.on_upgrade(|socket| chat_socket(socket, state, claims, pg, rd, connection_id, gate))
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
    state: ChatState,
    claims: Claims,
    pg: PgPool,
    mut rd: RdPool,
    connection_id: String,
    gate: Gate<Role, (Uuid, Uuid)>,
) {
    let mut controller = UserController::new(stream, claims.user_id, connection_id);

    loop {
        // Wait for next client action
        let action = controller.user_channel.receiver.next_action().await;
        match action {
            ClientAction::ChangeGroup { group_id } => {
                // Security checks
                if !connection_requirements(&pg, &group_id, &claims).await {
                    break;
                }

                // Fetch role and privileges in order to connect to group
                let Ok(privileges) = get_all_privileges(&pg, &mut rd, group_id).await else {
                    error!("Cannot fetch group role privileges");
                    continue;
                };
                let group_controller = state
                    .groups
                    .get(&group_id);

                let Ok(role) = get_user_role(&pg, &mut rd, claims.user_id, group_id).await else {
                    error!("Cannot fetch group user role data");
                    continue;
                };

                // Connect user controller to group
                controller.connect(group_id, group_controller, role).await;

                // Load last group messages
                let Ok(messages) = fetch_last_messages_in_range(&pg, &group_id, 10, 0).await
                else {
                    error!("ws closed: Cannot fetch group {} messages", &group_id);
                    continue;
                };

                // Send messages JSON object to user
                let payload = ServerAction::LoadMessages(messages);
                if controller.user_channel.sender.send(&payload).await.is_err() {
                    error!("ws closed: Failed to load fetched messages");
                    continue;
                }
            }
            ClientAction::SendMessage { content } => {
                let Some(conn) = controller.get_group_conn().await else {
                    debug!(
                        "Cannot send message from user {} - group not selected",
                        &claims.user_id, 
                    );
                    continue;
                };

                // Forbid too long messages
                if content.len() > MAX_MESSAGE_LENGTH {
                    debug!(
                        "Message too long: the message length is {}, which is greater than {}",
                        content.len(),
                        MAX_MESSAGE_LENGTH
                    );
                    continue;
                }

                // todo: make transaction
                // Save message in database
                let nickname = get_group_nickname(&pg, &claims.user_id, &conn.group_id)
                    .await
                    .unwrap_or("unknown_user".into());

                let Ok(_) = create_message(&pg, &claims.user_id, &conn.group_id, &content).await
                else {
                    error!(
                        "Failed to save the message from the user {} in the database",
                        &claims.user_id, 
                    );
                    continue;
                };

                // Send message to the connected group members
                let action = ServerAction::Message(GroupUserMessage::new(nickname, content));
                debug!("Sent: {action:#?}");
                conn.controller.channel.sender.send(action);
            }
            ClientAction::RequestMessages { loaded } => {
                let Some(conn) = controller.get_group_conn().await else {
                    debug!("Cannot fetch requested messages - group not selected");
                    continue;
                };
                info!("Requested messages");

                // Load older messages
                let Ok(messages) =
                    fetch_last_messages_in_range(&pg, &conn.group_id, 10, loaded).await
                else {
                    error!(
                        "ws closed: Cannot fetch group messages for user {}",
                        &claims.user_id, 
                    );
                    continue;
                };

                // Send messages json object
                let payload = ServerAction::LoadRequested(messages);
                if controller.user_channel.sender.send(&payload).await.is_err() {
                    error!(
                        "Failed to load messages for user {}",
                        &claims.user_id, 
                    );
                    continue;
                }
            }
            // todo: send group invites in chat
            ClientAction::GroupInvite { group_id } => {
                todo!();
                // match controller
                //     .verify_with_privilege(claims.user_id, Privilege::CanInvite(CanInvite::Yes))
                //     .await
                // {
                //     Ok(false) => {
                //         info!("User does not have privileges to invite other users");
                //         continue;
                //     }
                //     Err(e) => {
                //         error!("Failed to verify with privilege: {:?}", e);
                //         continue;
                //     }
                //     _ => (),
                // }

                let Ok(_is_member) = check_if_group_member(&pg, &claims.user_id, &group_id).await
                else {
                    error!("Failed to check whether a user {} is a group {} member (during sending a group invite)", &claims.user_id, &group_id);
                    continue;
                };
            }
            ClientAction::RemoveUser { user_id, group_id } => {
                match check_if_group_member(&pg, &user_id, &group_id).await {
                    Ok(false) => {
                        debug!(
                            "Cannot remove user {} from group {} - user is not a group member",
                            &user_id, &group_id
                        );
                        continue;
                    }
                    Err(_) => {
                        error!("ws closed: Failed to check whether a user {} is a group {} member (during user removal)", &user_id, &group_id);
                        continue;
                    }
                    _ => (),
                };

                todo!();
                // let Some(user_role) = controller.get_role(claims.user_id).await else {
                //     error!("Failed to get the controller's role");
                //     continue;
                // };

                // let Some(target_user_role) = controller.get_role(user_id).await else {
                //     error!("Failed to get the target user's role");
                //     continue;
                // };

                // if !gate.verify(user_role, target_user_role, (claims.user_id, user_id)) {
                //     info!("User does not have privileges to kick another user");
                //     continue;
                // }

                // Remove user from group
                let Ok(_) = try_remove_user_from_group(&pg, user_id, group_id).await else {
                    error!(
                        "Failed to remove user {} from a group {}",
                        &user_id, &group_id
                    );
                    continue;
                };

                // Stop listening for new group messages on all kicked user connections
                controller.kick(user_id).await;

                // todo: disconnect group controllers
            }
            ClientAction::Close => {
                info!("WebSocket closed explicitly");
                break;
            }
            ClientAction::Ignore => {
                info!("Action can't be handled");
                continue;
            }
        }
    }

    debug!("ws closed: User left the message loop");
    controller.disconnect().await;
}

/// Checks if group exsists and if users is a group member
async fn connection_requirements(pool: &PgPool, group_id: &Uuid, claims: &Claims) -> bool {
    let Ok(is_group) = check_if_group_exists(pool, group_id).await else {
        error!("ws closed: Cannot check if group {} exists", group_id);
        return false;
    };
    if !is_group {
        info!("ws closed: Non existing group");
        return false;
    }
    let Ok(is_group_member) = check_if_group_member(pool, &claims.user_id, group_id).await else {
        error!(
            "ws closed: Cannot check if user {} is a group {} member",
            &claims.user_id, group_id
        );
        return false;
    };
    if !is_group_member {
        info!(
            "ws closed: User {} isn't a group member",
            &claims.user_id, 
        );
        return false;
    }
    true
}
