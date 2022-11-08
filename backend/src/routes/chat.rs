use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{Html, Response},
    Extension, Json,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde_json::{json, Value};
use sqlx::{query, PgPool, query_as};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tracing::{debug, error, info};
use uuid::Uuid;
use crate::models::MessageModel;

use crate::{models::Claims, queries::create_message};

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
            return;
        }
    };

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
        return
    };

    if query!(
        r#"
        select * from groups
        where id = $1
    "#,
        group_id
    )
    .fetch_one(&mut conn)
    .await
    .is_err()
    {
        return;
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
    .await
    .is_err()
    {
        return;
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

    let Ok(res) = query_as!(
        MessageModel,
        r#"
            select * from messages
            where group_id = $1
        "#,
        group_id
    )
    .fetch_all(&mut conn)
    .await else {
        error!("Cannot fetch messages from database");
        return;
    };

    for record in res.iter() {
        if sender.send(Message::Text(format!("{}", record.content))).await.is_err() {
            error!("Failed to load messages");
            break;
        }
    }

    // Subscribe before sending joined message.
    let (tx, mut rx) = {
        let mut groups = state.groups.lock().unwrap();

        let group = groups
            .entry(group_id)
            .and_modify(|val| {
                val.users.insert(claims.id);
            })
            .or_insert(GroupTransmitter::new());

        let rx = group.tx.subscribe();

        // Send joined message to all subscribers.
        let msg = format!("{} joined.", username);
        debug!("{}", msg);
        let _ = group.tx.send(msg);

        (group.tx.clone(), rx)
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

            if create_message(&mut conn, claims.id, group_id, &msg).await.is_err() {
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
    pool: Extension<PgPool>,
) -> Result<Json<Value>, StatusCode> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let res = query!(
        r#"
        select groups.id, groups.name from group_users
        join groups on groups.id = group_users.group_id
        where user_id = $1
        "#,
        claims.id
    )
    .fetch_all(&mut conn)
    .await;

    match res {
        Ok(groups) => {
            debug!("{groups:#?}");
            let groups = groups
                .into_iter()
                .map(|group| (group.name, group.id))
                .collect::<Vec<(String, Uuid)>>();
            debug!("{groups:#?}");
            Ok(Json(json!({ "groups": groups })))
        }
        Err(e) => {
            match e {
                sqlx::Error::RowNotFound => return Ok(Json(json!({"groups": []}))),
                _ => return Err(StatusCode::INTERNAL_SERVER_ERROR),
            };
        }
    }
}
// Include utf-8 file at **compile** time.
pub async fn chat_index() -> Html<&'static str> {
    Html(std::include_str!("../../chat.html"))
}
