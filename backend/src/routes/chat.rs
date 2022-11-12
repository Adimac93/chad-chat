﻿use crate::models::{ChatState, Claims};
use crate::utils::chat::*;
use crate::utils::groups::*;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Json,
    response::Response,
    routing::{get, post},
    Extension, Router,
};
use futures::{SinkExt, StreamExt};
use sqlx::{PgPool, Pool, Postgres};
use std::str::FromStr;
use std::sync::Arc;
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

async fn chat_socket(stream: WebSocket, state: Arc<ChatState>, claims: Claims, pool: PgPool) {
    // By splitting we can send and receive at the same time.
    let (mut sender, mut receiver) = stream.split();

    // Loop until a text message is found.
    let mut group_id = String::new();
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(id) = message {
            info!("Group id: {}", id);
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

    let Ok(is_group_member) = check_if_group_member(&pool,&claims.id,&group_id).await else {
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

    // Subscribe before sending "joined" message.
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
                error!("Failed to save this message in the database")
            }
        }
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        _ = (&mut send_task_to_client) => recv_task_from_client.abort(),
        _ = (&mut recv_task_from_client) => send_task_to_client.abort(),
    };

    // Send "user left" message.
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