use crate::{
    chat::{
        check_if_group_exists, check_if_is_group_member, fetch_chat_messages, get_user_login_by_id,
        subscribe, ChatError, ChatState,
    },
    models::{Group},
};
use anyhow::Context;
use axum::{
    debug_handler,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{Html, Response},
    Extension, Json,
};
use futures::{StreamExt, SinkExt};
use serde_json::{json, Value};
use sqlx::{PgPool, query_as, Pool, Postgres};
use std::{
    sync::{Arc},
};
use std::str::FromStr;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{models::Claims, queries::create_message};

pub async fn chat_handler(
    ws: WebSocketUpgrade,
    claims: Claims,
    Extension(state): Extension<Arc<ChatState>>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    ws.on_upgrade(|socket| chat_socket(socket, state, claims, pool))
}

async fn chat_socket(
    stream: WebSocket,
    state: Arc<ChatState>,
    claims: Claims,
    pool: Pool<Postgres>,
) {
    // By splitting we can send and receive at the same time.
    let (mut sender, mut receiver) = stream.split();

    // Loop until a text message is found.
    let mut group_id = String::new();
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(id) = message {
            info!("Valid group id: {}", id);
            group_id = id;
            break;
        }
    }

    let Ok(group_id) = Uuid::from_str(&group_id) else {
        error!("Provided invalid UUID");
        return;
    };

    let Ok(is_group) = check_if_group_exists(&pool,&group_id).await else {
        error!("Cannot check if group exists");
        return;
    };

    if !is_group {
        info!("Non existing group");
        return;
    }

    let Ok(is_group_member) = check_if_is_group_member(&pool,&group_id,&claims.id).await else {
        error!("Cannot check if user is a group member");
        return;
    };

    if !is_group_member {
        info!("User isn't a group member");
        return;
    }

    let Ok(username) = get_user_login_by_id(&pool,&claims.id).await else {
        error!("Cannot fetch user login by id");
        return;
    };

    let Ok(messages) = fetch_chat_messages(&pool,&group_id).await else {
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

    // Subscribe before sending joined message.
    let (tx, mut rx) = {
        let mut groups = state.groups.lock().unwrap();
        subscribe(&mut groups, group_id, claims.id, &username)
    };

    // This task will receive broadcast messages and send text message to our client.
    let mut send_task_to_client = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // In any websocket error, break loop.
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Clone things we want to pass to the receiving task.
    let name = username.clone();
    let cloned_tx = tx.clone();

    // This task will receive messages from client and send them to broadcast subscribers.
    let mut recv_task_from_client = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // Add username before message.
            let msg = format!("{}: {}", name, text);
            let _ = cloned_tx.send(msg.clone());

            if create_message(&pool, &claims.id, &group_id, &msg)
                .await
                .is_err()
            {
                error!("Failed to add the message to the database")
            }
        }
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        _ = (&mut send_task_to_client) => recv_task_from_client.abort(),
        _ = (&mut recv_task_from_client) => send_task_to_client.abort(),
    };

    // Send user left message.
    let msg = format!("{} left.", username);
    debug!("{}", msg);
    let _ = tx.send(msg);
    {
        let mut groups = state.groups.lock().unwrap();
        groups.entry(group_id).and_modify(|group| {
            let _is_present = group.users.remove(&claims.id);
        });
    }
}

pub async fn get_user_groups(
    claims: Claims,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Value>, ChatError> {
    let groups = query_as!(
        Group,
        r#"
        select groups.id, groups.name from group_users
        join groups on groups.id = group_users.group_id
        where user_id = $1
        "#,
        claims.id
    )
    .fetch_all(&pool)
    .await
    .context("Failed to select groups with provided user id")?;

    Ok(Json(json!({ "groups": groups })))
}
// Include utf-8 file at **compile** time.
pub async fn chat_index() -> Html<&'static str> {
    Html(std::include_str!("../../chat.html"))
}
