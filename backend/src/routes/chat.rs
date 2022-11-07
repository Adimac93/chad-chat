use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{Html, Response},
    Extension,
};
use futures::{sink::SinkExt, stream::StreamExt};
use sqlx::{query, query_as, PgPool};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::models::Claims;

// Our shared state
pub struct AppState {
    user_set: Mutex<HashSet<String>>,
    tx: broadcast::Sender<String>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            user_set: Mutex::new(HashSet::new()),
            tx,
        }
    }
}

pub async fn chat_handler(
    ws: WebSocketUpgrade,
    state: Extension<Arc<AppState>>,
    claims: Claims,
    pool: Extension<PgPool>,
) -> Response {
    ws.on_upgrade(|socket| chat_socket(socket, state, claims, pool))
}

async fn chat_socket(
    stream: WebSocket,
    state: Extension<Arc<AppState>>,
    claims: Claims,
    pool: Extension<PgPool>,
) {
    // By splitting we can send and receive at the same time.
    let (mut sender, mut receiver) = stream.split();

    // Username gets set in the receive loop, if it's valid.
    let mut conn = match pool.acquire().await {
        Ok(conn) => conn,
        Err(e) => return,
    };
    let Ok(res) = query!(
        r#"
            select login from users where id = $1
        "#,
        claims.id
    )
    .fetch_one(&mut conn)
    .await else {
        return;
    };

    let username = res.login;

    // Subscribe before sending joined message.
    let mut rx = state.tx.subscribe();

    // Send joined message to all subscribers.
    let msg = format!("{} joined.", username);
    tracing::debug!("{}", msg);
    let _ = state.tx.send(msg);

    // This task will receive broadcast messages and send text message to our client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // In any websocket error, break loop.
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Clone things we want to pass to the receiving task.
    let tx = state.tx.clone();
    let name = username.clone();

    // This task will receive messages from client and send them to broadcast subscribers.
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // Add username before message.
            let _ = tx.send(format!("{}: {}", name, text));
        }
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    // Send user left message.
    let msg = format!("{} left.", username);
    tracing::debug!("{}", msg);
    let _ = state.tx.send(msg);
    // Remove username from map so new clients can take it.
    state.user_set.lock().unwrap().remove(&username);
}

fn check_username(state: &AppState, string: &mut String, name: &str) {
    let mut user_set = state.user_set.lock().unwrap();

    if !user_set.contains(name) {
        user_set.insert(name.to_owned());

        string.push_str(name);
    }
}

// Include utf-8 file at **compile** time.
pub async fn chat() -> Html<&'static str> {
    Html(std::include_str!("../../index.html"))
}
