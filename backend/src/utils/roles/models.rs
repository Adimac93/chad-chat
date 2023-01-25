use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    hash::Hash, cmp::Ordering,
};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{errors::RoleError, privileges::{Privileges, Privilege, CanInvite, CanSendMessages}};

#[derive(
    sqlx::Type, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy,
)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Member,
    Admin,
    Owner,
}

impl Role {
    fn increment(self) -> Option<Self> {
        match self {
            Role::Member => Some(Role::Admin),
            Role::Admin => Some(Role::Owner),
            Role::Owner => None,
        }
    }

    fn decrement(self) -> Option<Self> {
        match self {
            Role::Member => None,
            Role::Admin => Some(Role::Member),
            Role::Owner => Some(Role::Admin),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct GroupRolePrivileges(pub HashMap<Role, Privileges>);

#[derive(Clone)]
pub struct SocketGroupRolePrivileges(pub HashMap<Role, Arc<RwLock<Privileges>>>);

impl SocketGroupRolePrivileges {
    pub async fn get_privileges(&self, role: Role) -> Option<Privileges> {
        if role == Role::Owner {
            Some(Privileges::max())
        } else {
            Some(self.0.get(&role)?.read().await.clone())
        }
    }
}

impl GroupRolePrivileges {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl From<GroupRolePrivileges> for SocketGroupRolePrivileges {
    fn from(val: GroupRolePrivileges) -> Self {
        SocketGroupRolePrivileges(
            val.0
                .into_iter()
                .map(|(k, v)| (k, Arc::new(RwLock::new(v))))
                .collect::<HashMap<_, _>>(),
        )
    }
}

impl Privilege {
    fn partial_cmp_max(self, other: Self) -> Option<Self> {
        match &self.partial_cmp(&other) {
            Some(Ordering::Greater) | Some(Ordering::Equal) => Some(self),
            Some(Ordering::Less) => Some(other),
            None => None,
        }
    }

    fn partial_cmp_min(self, other: Self) -> Option<Self> {
        match &self.partial_cmp(&other) {
            Some(Ordering::Greater) | Some(Ordering::Equal) => Some(other),
            Some(Ordering::Less) => Some(self),
            None => None,
        }
    }
}

impl PartialOrd for Privilege {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            Privilege::CanInvite(x) => match other {
                Privilege::CanInvite(y) => x.partial_cmp(y),
                _ => None,
            },
            Privilege::CanSendMessages(x) => match other {
                Privilege::CanSendMessages(y) => x.partial_cmp(y),
                _ => None,
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct PrivilegeChangeData {
    pub group_id: Uuid,
    pub role: Role,
    pub value: Privilege,
}

impl PrivilegeChangeData {
    pub fn new(group_id: Uuid, role: Role, value: Privilege) -> Self {
        Self { group_id, role, value }
    }
}

impl PrivilegeChangeData {
    pub async fn maintain_hierarchy(
        &mut self,
        other: &SocketGroupRolePrivileges,
    ) -> Result<(), RoleError> {
        self.maintain_hierarchy_l(other).await?;
        self.maintain_hierarchy_h(other).await?;
        Ok(())
    }

    pub async fn maintain_hierarchy_l(
        &mut self,
        other: &SocketGroupRolePrivileges,
    ) -> Result<(), RoleError> {
        let Some(other_role) = self.role.decrement() else { return Ok(()) };

        let other_privileges_ref = other.0.get(&other_role).ok_or(RoleError::RoleNotFound)?;
        let other_privileges = other_privileges_ref.read().await;
        let privilege = other_privileges.0.get(&self.value).ok_or(RoleError::Unexpected(anyhow!("Privilege not found")))?;
        
        self.value = self.value.partial_cmp_max(*privilege).ok_or(RoleError::Unexpected(anyhow!("Mismatched privileges")))?;

        Ok(())
    }

    pub async fn maintain_hierarchy_h(
        &mut self,
        other: &SocketGroupRolePrivileges,
    ) -> Result<(), RoleError> {
        let Some(other_role) = self.role.increment() else { return Ok(()) };
        if other_role == Role::Owner { return Ok(()) }

        let other_privileges_ref = other.0.get(&other_role).ok_or(RoleError::RoleNotFound)?;
        let other_privileges = other_privileges_ref.read().await;
        let privilege = other_privileges.0.get(&self.value).ok_or(RoleError::Unexpected(anyhow!("Privilege not found")))?;

        self.value = self.value.partial_cmp_min(*privilege).ok_or(RoleError::Unexpected(anyhow!("Mismatched privileges")))?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserRoleChangeData {
    pub group_id: Uuid,
    pub user_id: Uuid,
    pub value: Role,
}

impl UserRoleChangeData {
    pub fn new(group_id: Uuid, user_id: Uuid, value: Role) -> Self {
        Self { group_id, user_id, value }
    }
}

#[derive(Debug)]
pub struct PrivilegeInterpretationData {
    pub can_invite: bool,
    pub can_send_messages: i32,
}

impl PrivilegeInterpretationData {
    pub fn new(can_invite: bool, can_send_messages: i32) -> Self {
        Self { can_invite, can_send_messages }
    }
}

impl TryFrom<PrivilegeInterpretationData> for Privileges {
    type Error = RoleError;

    fn try_from(val: PrivilegeInterpretationData) -> Result<Self, Self::Error> {
        let mut res = Privileges::new();
        res.0.insert(Privilege::CanInvite(CanInvite::from(val.can_invite)));
        res.0.insert(Privilege::CanSendMessages(CanSendMessages::try_from(val.can_send_messages)?));

        Ok(res)
    }
}
