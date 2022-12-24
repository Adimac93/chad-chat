use super::socket::{GroupConnection, GroupSender, ServerAction, UserConnection, UserSender};
use axum::extract::ws::WebSocket;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};
use std::{collections::HashMap, sync::Arc};
use time::OffsetDateTime;
use tokio::{sync::RwLock, task::JoinHandle};
use uuid::Uuid;

pub struct ChatState {
    pub groups: Groups,
}

pub struct UserController {
    user_id: Uuid,
    connection_id: String,
    pub user_conn: UserConnection,
    pub group_controller: Option<GroupController>, // should not be owned by struct (maybe RwLock)
}

impl UserController {
    pub fn new(stream: WebSocket, user_id: Uuid, connection_id: String) -> Self {
        Self {
            user_id,
            user_conn: UserConnection::new(stream),
            connection_id,
            group_controller: None,
        }
    }

    pub fn get_group_controller(&self) -> &Option<GroupController> {
        &self.group_controller
    }
}

pub struct GroupController {
    pub group_id: Uuid,
    pub sender: GroupSender,
}

impl GroupController {
    fn new(group_id: Uuid, sender: GroupSender) -> Self {
        Self { group_id, sender }
    }
}
pub struct Groups(DashMap<Uuid, GroupUserConnections>);

impl Groups {
    fn new() -> Self {
        Self(DashMap::new())
    }

    pub async fn connect(&self, controller: &mut UserController, group_id: &Uuid) {
        let Groups(groups) = self;
        let mut group = groups
            .entry(*group_id)
            .or_insert(GroupUserConnections::new());

        let group_conn = group.conn.emit();

        // Start listening for new group messages
        let conn = controller
            .user_conn
            .sender
            .listen(group_conn.receiver)
            .await;

        // Mount group controller
        controller.group_controller = Some(GroupController::new(*group_id, group_conn.sender));

        // Register connection
        group
            .users
            .connect(&controller.user_id, conn, controller.connection_id.clone())
            .await;
    }

    pub async fn disconnect(&self, controller: &UserController) -> bool {
        if let Some(group_controller) = &controller.get_group_controller() {
            let Groups(groups) = self;
            if let Some(mut group) = groups.get_mut(&group_controller.group_id) {
                group
                    .users
                    .disconnect(&controller.user_id, controller.connection_id.clone())
                    .await;
                return true;
            }
        }
        false
    }

    pub async fn kick(
        &self,
        pool: &PgPool,
        user_id: &Uuid,
        group_id: &Uuid,
        kick_message: KickMessage,
    ) -> bool {
        let Groups(groups) = self;
        if let Some(mut group) = groups.get_mut(group_id) {
            group.users.disconnect_all(user_id, kick_message).await;
            // todo: remove group controller for kicked user
        }
        query!(
            r#"
                delete from group_users
                where group_id = $1 and user_id = $2
            "#,
            group_id,
            user_id
        )
        .execute(pool)
        .await
        .is_ok()
    }
}

impl ChatState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            groups: Groups::new(),
        })
    }
}

pub struct GroupUserConnections {
    conn: GroupConnection,
    users: ChatUsers,
}

impl GroupUserConnections {
    fn new() -> Self {
        Self {
            conn: GroupConnection::new(100),
            users: ChatUsers::new(),
        }
    }
}

pub struct ChatUsers(DashMap<Uuid, UserConnections>);

impl ChatUsers {
    fn new() -> Self {
        Self(DashMap::new())
    }

    async fn disconnect(&mut self, user_id: &Uuid, connection_id: String) {
        let ChatUsers(group_users) = self;
        if let Some((_, user_connections)) = group_users.remove(user_id) {
            if let Some((_, task)) = user_connections.remove(connection_id).await {
                task.abort();
            }
        }
    }

    async fn disconnect_all(&mut self, user_id: &Uuid, kick_message: KickMessage) {
        let ChatUsers(group_users) = self;
        if let Some(user_connections) = group_users.get_mut(user_id) {
            if let Some(user_connecetions) = user_connections.remove_all().await {
                for (sender, task) in user_connecetions {
                    task.abort();
                    sender.send(&ServerAction::Kick(kick_message.clone())).await;
                }
            }
        }
    }

    async fn connect(
        &mut self,
        user_id: &Uuid,
        conn: (UserSender, JoinHandle<()>),
        connection_id: String,
    ) {
        let ChatUsers(group_users) = self;
        if let Some(user_connections) = group_users.get_mut(user_id) {
            user_connections.add(connection_id, conn).await;
        }
    }
}

pub struct UserConnections(RwLock<HashMap<String, (UserSender, JoinHandle<()>)>>);

impl UserConnections {
    fn new() -> Self {
        Self(RwLock::new(HashMap::new()))
    }

    async fn add(&self, connection_id: String, conn: (UserSender, JoinHandle<()>)) {
        let UserConnections(connections) = self;
        connections
            .write()
            .await
            .entry(connection_id)
            .or_insert(conn);
    }

    async fn remove(&self, connection_id: String) -> Option<(UserSender, JoinHandle<()>)> {
        let UserConnections(connections) = self;
        connections.write().await.remove(&connection_id)
    }

    async fn remove_all(&self) -> Option<Vec<(UserSender, JoinHandle<()>)>> {
        let UserConnections(connections) = self;
        if connections.read().await.is_empty() {
            return None;
        }
        Some(
            connections
                .write()
                .await
                .drain()
                .map(|(_, conn)| conn)
                .collect::<Vec<(UserSender, JoinHandle<()>)>>(),
        )
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct AddresedMessage {
    pub content: String,
    pub user_id: Uuid,
    pub group_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GroupUserMessageModel {
    pub nickname: String,
    pub content: String,
    pub sent_at: OffsetDateTime,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GroupUserMessage {
    pub nickname: String,
    pub content: String,
    pub sat: i64,
}

impl GroupUserMessage {
    pub fn new(nickname: String, content: String) -> Self {
        Self {
            nickname,
            content,
            sat: OffsetDateTime::now_utc().unix_timestamp(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KickMessage {
    from: String,
    reason: String,
}
