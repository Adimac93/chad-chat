use crate::state::AppState;
use crate::utils::auth::models::Claims;
use crate::utils::chat::messages::fetch_last_messages_in_range;
use crate::utils::chat::models::*;
use crate::utils::chat::socket::{ChatState, ClientAction, ServerAction, UserController};
use crate::utils::chat::*;
use crate::utils::groups::*;
use crate::utils::roles::models::{Gate, Role, SocketGroupRolePrivileges};
use crate::utils::roles::privileges::{CanInvite, Privilege};
use crate::utils::roles::{
    get_group_role_privileges, get_user_role, single_set_group_role_privileges,
    single_set_group_user_role,
};
use anyhow::Context;
use axum::extract::State;
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

macro_rules! skip_error {
    ($result:expr) => {{
        match $result {
            Ok(value) => value,
            Err(error) => {
                tracing::error!("{error}");
                continue;
            }
        }
    }};
}

async fn chat_handler(
    ws: WebSocketUpgrade,
    claims: Claims,
    State(state): State<ChatState>,
    State(pool): State<PgPool>,
    State(gate): State<Gate<Role, (Uuid, Uuid)>>,
) -> Response {
    ws.on_upgrade(|socket| chat_socket(socket, state, claims, pool, gate))
}

pub async fn chat_socket(
    stream: WebSocket,
    state: ChatState,
    claims: Claims,
    pool: PgPool,
    gate: Gate<Role, (Uuid, Uuid)>,
) {
    let mut controller = UserController::new(stream, claims.user_id);

    loop {
        // Wait for next client action
        let action = controller.user_channel.receiver.next_action().await;
        match action {
            ClientAction::ChangeGroup { group_id } => {
                // Security checks
                if !connection_requirements(&pool, &group_id, &claims).await {
                    break;
                }

                // Fetch role and privileges in order to connect to group
                let privileges = skip_error!(get_group_role_privileges(&pool, group_id)
                    .await
                    .context("Failed to get group role privileges"));

                let group_controller = state
                    .groups
                    .get(&group_id, SocketGroupRolePrivileges::from(privileges));

                let role = skip_error!(get_user_role(&pool, &claims.user_id, &group_id)
                    .await
                    .context("Cannot fetch group user role data"));

                // Connect user controller to group
                controller.connect(group_id, group_controller, role).await;

                // Load last group messages
                let messages = skip_error!(fetch_last_messages_in_range(&pool, &group_id, 10, 0)
                    .await
                    .context(format!(
                        "ws closed: Cannot fetch group {} messages",
                        &group_id
                    )));

                // Send messages JSON object to user
                let payload = ServerAction::LoadMessages(messages);
                skip_error!(controller
                    .user_channel
                    .sender
                    .send(&payload)
                    .await
                    .context("ws closed: Failed to load fetched messages"));
            }
            ClientAction::SendMessage { content } => {
                let conn = skip_error!(controller.get_group_conn().await.context(format!(
                    "Cannot send message from user {} - group not selected",
                    &claims.user_id,
                )));

                // Forbid too long messages
                if content.len() > MAX_MESSAGE_LENGTH {
                    debug!(
                        "Message too long: the message length is {}, which is greater than {}",
                        content.len(),
                        MAX_MESSAGE_LENGTH
                    );
                    continue;
                }

                let nickname = get_group_nickname(&pool, &claims.user_id, &conn.group_id)
                    .await
                    .unwrap_or("unknown_user".into());

                create_message(&pool, &claims.user_id, &conn.group_id, &content)
                    .await
                    .expect("Failed to create a message");

                // Send message to the connected group members
                let action = ServerAction::Message(GroupUserMessage::new(nickname, content));
                conn.controller.channel.sender.send(action);
            }
            ClientAction::RequestMessages { loaded } => {
                let conn = skip_error!(controller
                    .get_group_conn()
                    .await
                    .context("Cannot fetch requested messages - group not selected"));

                info!("Requested messages");

                // Load older messages
                let messages =
                    skip_error!(
                        fetch_last_messages_in_range(&pool, &conn.group_id, 10, loaded)
                            .await
                            .context(format!(
                                "cannot fetch group messages for user {}",
                                &claims.user_id
                            ))
                    );

                // Send messages json object
                let payload = ServerAction::LoadRequested(messages);
                skip_error!(controller
                    .user_channel
                    .sender
                    .send(&payload)
                    .await
                    .context(format!(
                        "Failed to load messages for user {}",
                        &claims.user_id
                    )));
            }
            // todo: send group invites in chat
            ClientAction::GroupInvite { group_id } => {
                match controller
                    .verify_with_privilege(claims.user_id, Privilege::CanInvite(CanInvite::Yes))
                    .await
                {
                    Ok(false) => {
                        info!("User does not have privileges to invite other users");
                        continue;
                    }
                    Err(e) => {
                        error!("Failed to verify with privilege: {:?}", e);
                        continue;
                    }
                    _ => (),
                }

                let _is_member = skip_error!(check_if_group_member(&pool, &claims.user_id, &group_id).await.context(format!("Failed to check whether a user {} is a group {} member (during sending a group invite)", &claims.user_id, &group_id)));
            }
            ClientAction::RemoveUser { user_id, group_id } => {
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
                        continue;
                    }
                    _ => (),
                };

                let user_role = skip_error!(controller
                    .get_role(claims.user_id)
                    .await
                    .context("Failed to get the controller's role"));

                let target_user_role = skip_error!(controller
                    .get_role(user_id)
                    .await
                    .context("Failed to get the target user's role"));

                if !gate.verify(user_role, target_user_role, (claims.user_id, user_id)) {
                    info!("User does not have privileges to kick another user");
                    continue;
                }

                // Remove user from group
                skip_error!(try_remove_user_from_group(&pool, user_id, group_id)
                    .await
                    .context(format!(
                        "Failed to remove user {} from a group {}",
                        &user_id, &group_id
                    )));

                // Stop listening for new group messages on all kicked user connections
                controller.kick(user_id).await;

                // todo: disconnect group controllers
            }
            ClientAction::SingleChangePrivileges { mut data } => {
                let socket_privileges = skip_error!(controller
                    .get_group_privileges()
                    .context("User trying to change privileges not in group"));

                // there is a concurrency-related edge case which bypasses corrections
                skip_error!(data
                    .maintain_hierarchy(socket_privileges)
                    .await
                    .context("Error when maintaining role hierarchy"));

                skip_error!(controller
                    .set_privilege(&data)
                    .await
                    .context("Error when changing privilege"));

                skip_error!(single_set_group_role_privileges(&pool, &data)
                    .await
                    .context("Error when setting group role privileges"));
            }
            ClientAction::SingleChangeUserRole { data } => {
                skip_error!(controller.single_set_role(&data).await);
                skip_error!(single_set_group_user_role(&pool, &data)
                    .await
                    .context("Failed to change user role"));
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
        info!("ws closed: User {} isn't a group member", &claims.user_id,);
        return false;
    }
    true
}
