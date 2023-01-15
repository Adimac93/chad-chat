use crate::utils::roles::errors::RoleError;
use crate::utils::roles::models::{Privileges, Role, GroupUsersRole, SocketGroupRolePrivileges, BulkNewGroupRolePrivileges, PrivilegeChangeData};

use super::models::{GroupUserMessage, KickMessage};
use anyhow::anyhow;
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::select;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast;
use tokio::sync::{Mutex, Notify, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace};
use uuid::Uuid;

pub struct ChatState {
    pub groups: Groups,
}

impl ChatState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            groups: Groups::new(),
        })
    }
}

pub struct Groups(DashMap<Uuid, GroupController>);
impl Groups {
    fn new() -> Self {
        Self(DashMap::new())
    }

    pub fn get(&self, group_id: &Uuid, privileges: SocketGroupRolePrivileges) -> GroupController {
        let Groups(groups) = self;
        groups
            .entry(*group_id)
            .or_insert(GroupController::new(100, privileges))
            .value()
            .clone()
    }
}

#[derive(Clone)]
pub struct GroupController {
    pub channel: GroupChannel,
    users: Users,
    privileges: SocketGroupRolePrivileges,
}

impl GroupController {
    fn new(capacity: usize, privileges: SocketGroupRolePrivileges) -> Self {
        Self {
            channel: GroupChannel::new(capacity),
            users: Users::new(),
            privileges: privileges,
        }
    }
}

#[derive(Clone)]
struct Users(Arc<RwLock<HashMap<Uuid, GroupUserData>>>);
impl Users {
    fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
}
struct GroupUserData {
    role: Role,
    connections: UserConnections,
}

impl GroupUserData {
    fn new(role: Role) -> Self {
        Self {
            role,
            connections: UserConnections::new(),
        }
    }
}

struct UserConnections(Arc<RwLock<HashMap<String, UserChannelListener>>>);
impl UserConnections {
    fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    async fn send_across_all(&self, msg: &ServerAction) {
        let guard = self.0.read().await;
        for (_, connection) in guard.iter() {
            connection.sender.send(msg).await;
        }
    }
}

pub struct UserController {
    user_id: Uuid,
    conn_id: String,
    pub user_channel: UserChannel,
    group_conn: Option<GroupConnection>,
}

pub struct GroupConnection {
    pub group_id: Uuid,
    pub controller: GroupController,
}

impl GroupConnection {
    fn new(group_id: Uuid, controller: GroupController) -> Self {
        Self {
            group_id,
            controller,
        }
    }
}
impl UserController {
    pub fn new(stream: WebSocket, user_id: Uuid, conn_id: String) -> Self {
        Self {
            user_id,
            conn_id,
            user_channel: UserChannel::new(stream),
            group_conn: None,
        }
    }

    pub async fn connect(&mut self, group_id: Uuid, group_controller: GroupController, role: Role) {
        if let None = self.group_conn {
            let listener = UserChannelListener::new(
                self.user_channel.sender.clone(),
                group_controller.channel.subscribe(),
            )
            .await;
            let listener = group_controller
                .users
                .0
                .write()
                .await
                .entry(self.user_id)
                .or_insert(GroupUserData::new(role))
                .connections
                .0
                .write()
                .await
                .insert(self.conn_id.clone(), listener);

            if let Some(prev_listener) = listener {
                prev_listener.disconnect();
            }

            self.group_conn = Some(GroupConnection {
                group_id,
                controller: group_controller,
            });
        }
    }

    pub async fn disconnect(&mut self) {
        if let Some(conn) = &self.group_conn {
            if let Some(user_data) = conn.controller.users.0.write().await.get(&self.user_id) {
                user_data.connections.0.write().await.remove(&self.conn_id);
            }
        }
    }

    pub async fn get_group_conn(&self) -> Option<&GroupConnection> {
        if let Some(conn) = &self.group_conn {
            if conn
                .controller
                .users
                .0
                .read()
                .await
                .get(&self.user_id)
                .is_some()
            {
                return Some(conn);
            }
        }
        None
    }

    pub async fn kick(&self, user_id: Uuid) {
        if let Some(conn) = &self.group_conn {
            if let Some(connections) = conn.controller.users.0.write().await.remove(&user_id) {
                let listeners: Vec<UserChannelListener> = connections
                    .connections
                    .0
                    .write()
                    .await
                    .drain()
                    .map(|(_, listener)| listener)
                    .collect();

                for listener in listeners {
                    listener
                        .disconnect_with_action(&ServerAction::Kick(KickMessage {
                            from: "somone".into(),
                            reason: "no reason".into(),
                        }))
                        .await;
                }
            }
        }
    }

    pub async fn bulk_set_privileges(&self, group_privileges: &BulkNewGroupRolePrivileges) {
        if let Some(conn) = &self.group_conn {
            for (role, new_privileges) in &group_privileges.0 {
                let Some(privilege_ref) =
                    conn.controller.privileges.0.get(&role) else {
                        error!("No role {role:?} found in a group");
                        continue
                    };

                let mut privilege_guard = privilege_ref.write().await;

                let users_guard = conn.controller.users.0.read().await;
                *privilege_guard = new_privileges.clone();

                // send new privileges to every user, whose privileges were changed
                for (_, data) in users_guard.iter() {
                    if data.role == *role {
                        data.connections.send_across_all(&ServerAction::SetPrivileges(new_privileges.clone())).await;
                    }
                }
            }
        }
    }

    pub async fn set_privilege(&self, data: &PrivilegeChangeData) -> Result<(), RoleError> {
        if let Some(conn) = &self.group_conn {
            let Some(privilege_ref) =
                conn.controller.privileges.0.get(&data.role) else {
                    return Err(RoleError::Unexpected(anyhow!("No role {:?} found in a group", &data.role)));
                };

            let mut privilege_guard = privilege_ref.write().await;
            
            match privilege_guard.0.get_mut(&data.privilege) {
                Some(privilege) => *privilege = data.value,
                _ => return Err(RoleError::Unexpected(anyhow!("No role {:?} found in a group", &data.role))),
            };

            let users_guard = conn.controller.users.0.read().await;

            // send new privileges to every user, whose privileges were changed
            for (_, user_data) in users_guard.iter() {
                if user_data.role == data.role {
                    user_data.connections.send_across_all(&ServerAction::SetPrivileges(privilege_guard.clone())).await;
                }
            }
        }

        Ok(())
    }

    pub async fn bulk_set_users_role(&self, new_roles: GroupUsersRole) {
        if let Some(conn) = &self.group_conn {
            let mut users_guard = conn.controller.users.0.write().await;
            for (role, users) in new_roles.0.into_iter() {
                for elem in users.into_iter() {
                    if let Some(user) = users_guard.get_mut(&elem.user_id) {
                        user.role = role;

                        let Some(privileges) = conn.controller.privileges.get_privileges(role).await else {
                            error!("No role {role:?} found in a group");
                            continue
                        };

                        // send new privileges to every user with changed role
                        user.connections.send_across_all(&ServerAction::SetPrivileges(privileges)).await;
                    };
                }
            };
        }
    }

    pub async fn get_role(&self) -> Option<Role> {
        let Some(conn) = &self.group_conn else {
            return None
        };

        match conn.controller.users.0.read().await.get(&self.user_id) {
            Some(user_data) => Some(user_data.role),
            None => None,
        }
    }

    pub fn get_privileges(&self) -> Option<&SocketGroupRolePrivileges> {
        let connection = self.group_conn.as_ref()?;
        Some(&connection.controller.privileges)
    }
}

pub struct UserChannelListener {
    task: JoinHandle<()>,
    sender: UserSender,
}

impl UserChannelListener {
    async fn new(sender: UserSender, broadcast_receiver: GroupReceiver) -> Self {
        // let notifier = Arc::new(Notify::new());
        let (task, sender) = sender
            .listen(broadcast_receiver)
            .await;
        Self {
            task,
            sender,
        }
    }

    pub fn disconnect(self) {
        self.task.abort();
    }

    // pub async fn send(&self, action: &ServerAction) {
    //     match (&self.task).await {
    //         Ok(sender) => {
    //             sender.send(action).await;
    //         },
    //         Err(e) => {
    //             error!("Error while accessing user sender: {e}");
    //         },
    //     }
    // }

    pub async fn disconnect_with_action(self, action: &ServerAction) {
        self.task.abort();
        self.sender.send(action).await;
    }
}

pub struct UserChannel {
    pub sender: UserSender,
    pub receiver: UserReceiver,
}

impl UserChannel {
    pub fn new(stream: WebSocket) -> Self {
        let (sender, receiver) = stream.split();
        Self {
            sender: UserSender::new(sender),
            receiver: UserReceiver::new(receiver),
        }
    }

    pub fn join(sender: UserSender, receiver: UserReceiver) -> Self {
        Self { sender, receiver }
    }

    pub fn split(self) -> (UserSender, UserReceiver) {
        (self.sender, self.receiver)
    }
}

#[derive(Clone)]
pub struct UserSender(Arc<Mutex<SplitSink<WebSocket, Message>>>);

impl UserSender {
    fn new(sender: SplitSink<WebSocket, Message>) -> Self {
        Self(Arc::new(Mutex::new(sender)))
    }

    /// Send server action to client
    pub async fn send(&self, action: &ServerAction) -> Result<(), axum::Error> {
        let UserSender(sender) = self;
        let msg = serde_json::to_string(action).unwrap();
        sender.lock().await.send(Message::Text(msg)).await
    }
    pub async fn listen(&self, broadcast_receiver: GroupReceiver) -> (JoinHandle<()>, UserSender) {
        let GroupReceiver(mut broadcast_receiver) = broadcast_receiver;

        let task_sender = self.clone();
        // Stop task on error or aborting
        let task = tokio::spawn(async move {
            while let Ok(action) = broadcast_receiver.recv().await {
                if task_sender.send(&action).await.is_err() {
                    error!("Error while sending message to the client");
                    break;
                }
            }
        });

        (task, self.clone())
    }

    /// Listen to receiver messages and pass them to client
    pub async fn listen_with_notifier(
        &self,
        broadcast_receiver: GroupReceiver,
        task_notifier: Arc<Notify>,
    ) -> JoinHandle<UserSender> {
        let GroupReceiver(mut broadcast_receiver) = broadcast_receiver;

        let task_sender = self.clone();
        // Stop task and return sender on: error, aborting, notification
        tokio::spawn(async move {
            let main_loop = async {
                while let Ok(action) = broadcast_receiver.recv().await {
                    if task_sender.send(&action).await.is_err() {
                        error!("Error while sending message to the client");
                        break;
                    }
                }
            };

            select! {
                _ = main_loop => {}
                _ = task_notifier.notified() => {}
            };
            return task_sender;
        })
    }
}

pub struct UserReceiver(SplitStream<WebSocket>);
impl UserReceiver {
    fn new(receiver: SplitStream<WebSocket>) -> Self {
        Self(receiver)
    }

    /// Get next client action
    pub async fn next_action(&mut self) -> ClientAction {
        let UserReceiver(receiver) = self;
        if let Some(conn) = receiver.next().await {
            return match conn {
                Ok(message) => ClientAction::new(message),
                Err(e) => {
                    debug!("Error while receiving message from stream {e}");
                    ClientAction::Ignore
                }
            };
        }
        debug!("Data stream dropped");
        ClientAction::Close
    }
}

/// Server action send to client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerAction {
    LoadMessages(Vec<GroupUserMessage>),
    LoadRequested(Vec<GroupUserMessage>),
    GroupInvite,
    Message(GroupUserMessage),
    Kick(KickMessage),
    SetPrivileges(Privileges),
}

/// Client action send to server
#[derive(Serialize, Deserialize)]
pub enum ClientAction {
    ChangeGroup { group_id: Uuid },
    SendMessage { content: String },
    GroupInvite { group_id: Uuid },
    RemoveUser { user_id: Uuid, group_id: Uuid },
    BulkChangePrivileges { group_id: Uuid, privileges: BulkNewGroupRolePrivileges },
    BulkChangeUsersRole { group_id: Uuid, users: GroupUsersRole },
    SingleChangePrivileges { data: PrivilegeChangeData },
    RequestMessages { loaded: i64 },
    Close,
    Ignore,
}

impl ClientAction {
    fn new(message: Message) -> Self {
        match message {
            Message::Text(text) => {
                serde_json::from_str::<ClientAction>(&text).unwrap_or(ClientAction::Ignore)
            }
            Message::Close(frame) => {
                match frame {
                    Some(frame) => {
                        trace!("Code: {} Reason: {}", frame.code, frame.reason);
                    }
                    None => {
                        trace!("Closed without frame")
                    }
                }
                debug!("Closing socket");
                ClientAction::Close
            }
            Message::Binary(_) => {
                info!("Binary message type is not supported");
                ClientAction::Ignore
            }
            Message::Ping(_) => {
                info!("Ping message type is not supported");
                ClientAction::Ignore
            }
            Message::Pong(_) => {
                info!("Pong message type is not supported");
                ClientAction::Ignore
            }
        }
    }
}

pub struct GroupChannel {
    pub sender: GroupSender,
    pub receiver: GroupReceiver,
}
impl Clone for GroupChannel {
    fn clone(&self) -> Self {
        self.emit()
    }
}

impl GroupChannel {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = broadcast::channel::<ServerAction>(capacity);
        Self {
            sender: GroupSender::new(sender),
            receiver: GroupReceiver::new(receiver),
        }
    }

    pub fn subscribe(&self) -> GroupReceiver {
        let GroupSender(sender) = &self.sender;
        GroupReceiver::new(sender.subscribe())
    }

    pub fn emit(&self) -> Self {
        let GroupSender(sender) = &self.sender;
        Self {
            sender: GroupSender::new(sender.clone()),
            receiver: GroupReceiver::new(sender.subscribe()),
        }
    }
}

pub struct GroupSender(broadcast::Sender<ServerAction>);

impl GroupSender {
    fn new(sender: broadcast::Sender<ServerAction>) -> Self {
        Self(sender)
    }

    /// Send server action to all group clients
    pub fn send(&self, action: ServerAction) {
        let GroupSender(sender) = self;
        let res = sender.send(action);
        match res {
            Ok(n) => {
                trace!("Action send to {n} group members");
            }
            Err(e) => {
                error!("Failed to execute server action for all active members: {e}");
            }
        }
    }
}

pub struct GroupReceiver(broadcast::Receiver<ServerAction>);

impl GroupReceiver {
    fn new(receiver: broadcast::Receiver<ServerAction>) -> Self {
        Self(receiver)
    }

    async fn next_action(&mut self) -> Result<ServerAction, RecvError> {
        return match self.0.recv().await {
            Ok(msg) => Ok(msg),
            Err(e) => {
                debug!("Recv error: {e}");
                Err(e)
            }
        };
    }
}
