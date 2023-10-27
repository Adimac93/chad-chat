use anyhow::anyhow;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap, hash::Hash, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::errors::AppError;
use super::privileges::{CanInvite, CanSendMessages, Privilege, Privileges};

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

    pub async fn get_privilege(&self, role: Role, val: Privilege) -> Option<Privilege> {
        self.0.get(&role)?.read().await.0.get(&val).copied()
    }

    pub async fn verify_with_privilege(
        &self,
        role: Role,
        min_val: Privilege,
    ) -> Result<bool, AppError> {
        let cmp_res = self
            .get_privilege(role, min_val)
            .await
            .ok_or(AppError::Unexpected(anyhow!("No privilege found")))?
            .partial_cmp(&min_val);
        Ok(cmp_res == Some(Ordering::Greater) && cmp_res == Some(Ordering::Equal))
    }
}

impl Default for GroupRolePrivileges {
    fn default() -> Self {
        Self::new()
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
        Self {
            group_id,
            role,
            value,
        }
    }
}

impl PrivilegeChangeData {
    pub async fn maintain_hierarchy(
        &mut self,
        other: &SocketGroupRolePrivileges,
    ) -> Result<(), AppError> {
        self.maintain_hierarchy_l(other).await?;
        self.maintain_hierarchy_h(other).await?;
        Ok(())
    }

    pub async fn maintain_hierarchy_l(
        &mut self,
        other: &SocketGroupRolePrivileges,
    ) -> Result<(), AppError> {
        let Some(other_role) = self.role.decrement() else {
            return Ok(());
        };

        let other_privileges_ref = other.0.get(&other_role).ok_or(AppError::exp(StatusCode::BAD_REQUEST, "Role not found in the group"))?;
        let other_privileges = other_privileges_ref.read().await;
        let privilege = other_privileges
            .0
            .get(&self.value)
            .ok_or(AppError::Unexpected(anyhow!("Privilege not found")))?;

        self.value = self
            .value
            .partial_cmp_max(*privilege)
            .ok_or(AppError::Unexpected(anyhow!("Mismatched privileges")))?;

        Ok(())
    }

    pub async fn maintain_hierarchy_h(
        &mut self,
        other: &SocketGroupRolePrivileges,
    ) -> Result<(), AppError> {
        let Some(other_role) = self.role.increment() else {
            return Ok(());
        };
        if other_role == Role::Owner {
            return Ok(());
        }

        let other_privileges_ref = other.0.get(&other_role).ok_or(AppError::exp(StatusCode::BAD_REQUEST, "Role not found in the group"))?;
        let other_privileges = other_privileges_ref.read().await;
        let privilege = other_privileges
            .0
            .get(&self.value)
            .ok_or(AppError::Unexpected(anyhow!("Privilege not found")))?;

        self.value = self
            .value
            .partial_cmp_min(*privilege)
            .ok_or(AppError::Unexpected(anyhow!("Mismatched privileges")))?;

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
        Self {
            group_id,
            user_id,
            value,
        }
    }
}

#[derive(Debug)]
pub struct PrivilegeInterpretationData {
    pub can_invite: bool,
    pub can_send_messages: i32,
}

impl PrivilegeInterpretationData {
    pub fn new(can_invite: bool, can_send_messages: i32) -> Self {
        Self {
            can_invite,
            can_send_messages,
        }
    }
}

impl TryFrom<PrivilegeInterpretationData> for Privileges {
    type Error = AppError;

    fn try_from(val: PrivilegeInterpretationData) -> Result<Self, Self::Error> {
        let mut res = Privileges::new();
        res.0
            .insert(Privilege::CanInvite(CanInvite::from(val.can_invite)));
        res.0
            .insert(Privilege::CanSendMessages(CanSendMessages::try_from(
                val.can_send_messages,
            )?));

        Ok(res)
    }
}

#[derive(Clone)]
pub struct Gate<T: Eq + Hash, U> {
    pub roles: HashMap<Role, i32>,
    pub requirements: HashMap<T, i32>,
    pub extra_condition: Option<fn(U) -> bool>,
}

impl<T: Eq + Hash, U> Gate<T, U> {
    pub fn build() -> GateBuilder<T, U> {
        GateBuilder {
            roles: HashMap::new(),
            requirements: HashMap::new(),
            extra_condition: None,
        }
    }

    pub fn verify(&self, role: Role, req: T, info: U) -> bool {
        let Some(amount_1) = self.roles.get(&role) else {
            return false;
        };
        let Some(amount_2) = self.requirements.get(&req) else {
            return false;
        };
        if amount_1 > amount_2 {
            return true;
        };
        if amount_1 < amount_2 {
            return false;
        };
        let Some(function) = self.extra_condition else {
            return false;
        };
        function(info)
    }
}

pub struct GateBuilder<T: Eq + Hash, U> {
    roles: HashMap<Role, i32>,
    requirements: HashMap<T, i32>,
    extra_condition: Option<fn(U) -> bool>,
}

impl<T: Eq + Hash, U> GateBuilder<T, U> {
    pub fn role(mut self, role: Role, val: i32) -> Self {
        self.roles.insert(role, val);
        self
    }

    pub fn req(mut self, req: T, val: i32) -> Self {
        self.requirements.insert(req, val);
        self
    }

    pub fn condition(mut self, con: fn(U) -> bool) -> Self {
        self.extra_condition = Some(con);
        self
    }

    pub fn finish(self) -> Gate<T, U> {
        Gate {
            roles: self.roles,
            requirements: self.requirements,
            extra_condition: self.extra_condition,
        }
    }
}

pub fn is_id_the_same(val: (Uuid, Uuid)) -> bool {
    val.0 == val.1
}
