use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{Html, Response},
    Extension,
};
use futures::{sink::SinkExt, stream::StreamExt};
use sqlx::{query, PgPool};
use uuid::Uuid;
use std::{
    sync::{Arc, Mutex}, str::FromStr, collections::{HashMap, HashSet},
};
use tokio::sync::broadcast;
use tracing::{error,debug};

use crate::models::Claims;

// Our shared state
pub struct AppState {
    groups: Mutex<HashMap<Uuid, GroupTransmitter>>,
}

struct GroupTransmitter {
    tx: broadcast::Sender<String>,
    users: HashSet<Uuid>,
}

impl GroupTransmitter {
    fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            tx,
            users: HashSet::new(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            groups: Mutex::new(HashMap::new()),
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
    Extension(state): Extension<Arc<AppState>>,
    claims: Claims,
    pool: Extension<PgPool>,
) {
    // By splitting we can send and receive at the same time.
    let (mut sender, mut receiver) = stream.split();

    let mut conn = match pool.acquire().await {
        Ok(conn) => conn,
        Err(e) => {
            error!("{e:?}");
            return
        },
    };

    // Loop until a text message is found.
    let mut group_id = String::new();
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(id) = message {
            group_id = id;
            break
        }
    }

    let Ok(group_id) = Uuid::from_str(&group_id) else {
        return
    };

    if query!(
    r#"
        select * from groups
        where id = $1
    "#,
    group_id)
    .fetch_one(&mut conn)
    .await.is_err() {
        return
    };

    if query!(
    r#"
        select * from group_users
        where group_id = $1
        and user_id = $2
    "#,
    group_id,
    claims.id
    )
    .fetch_one(&mut conn)
    .await.is_err() {
        return
    };

    let Ok(res) = query!(
        r#"
            select login from users where id = $1
        "#,
        claims.id
    )
    .fetch_one(&mut conn)
    .await else {
        error!("Cannot fetch user login from database");
        return;
    };

    let username = res.login;

    // Subscribe before sending joined message.
    let (tx, mut rx) =
    {
        let mut groups = state.groups.lock().unwrap();
        
        let group = groups.entry(group_id).and_modify(|val| {
            val.users.insert(claims.id);
        }).or_insert(GroupTransmitter::new());

        let rx = group.tx.subscribe();

        // Send joined message to all subscribers.
        let msg = format!("{} joined.", username);
        debug!("{}", msg);
        let _ = group.tx.send(msg);

        (group.tx.clone(), rx)
    };

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
    let name = username.clone();
    let cloned_tx = tx.clone();

    // This task will receive messages from client and send them to broadcast subscribers.
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // Add username before message.
            let _ = cloned_tx.send(format!("{}: {}", name, text));
        }
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    // Send user left message.
    let msg = format!("{} left.", username);
    debug!("{}", msg);
    let _ = tx.send(msg);
}

// Include utf-8 file at **compile** time.
pub async fn chat() -> Html<&'static str> {
    Html(std::include_str!("../../index.html"))
}
