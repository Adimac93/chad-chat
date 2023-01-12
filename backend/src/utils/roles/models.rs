use std::{cmp::Ordering, collections::{HashMap, HashSet}, sync::Arc};

use anyhow::Context;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;

use crate::utils::groups::models::GroupUser;

use super::errors::RoleError;

#[derive(sqlx::Type, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
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

impl TryFrom<String> for Role {
    type Error = RoleError;

    fn try_from(val: String) -> Result<Self, Self::Error> {
        match &*val {
            "owner" => Ok(Role::Owner),
            "admin" => Ok(Role::Admin),
            "member" => Ok(Role::Member),
            _ => Err(RoleError::RoleParseError),
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct GroupUsersRoleFromJson(pub HashMap<String, Vec<GroupUser>>);

#[derive(Clone, Debug, PartialEq)]
pub struct GroupUsersRole(pub HashMap<Role, Vec<GroupUser>>);

pub struct ChangedPrivilegeSet(pub HashSet<Role>);

impl GroupUsersRole {
    pub fn preprocess(&mut self, role: Role, user: GroupUser) -> Result<(), RoleError> {
        self.0.iter_mut().for_each(|(_, vec)| {
            vec.retain(|x| x.user_id != user.user_id);
        });
        self.0.retain(|_, vec| {!vec.is_empty()});

        if self.verify_before_role_change(role)? {
            self.0.entry(Role::Admin)
                .and_modify(|vec| vec.push(user))
                .or_insert(vec![user]);
        };

        Ok(())
    }

    fn verify_before_role_change(&mut self, role: Role) -> Result<bool, RoleError> {
        if role == Role::Member { return Err(RoleError::RoleChangeRejection) };
        if self.0.get(&Role::Owner).is_none() { return Ok(false) };
        let new_owners = self.0.get(&Role::Owner).unwrap();
        if new_owners.is_empty() { Ok(false) }
        else if new_owners.len() == 1 && role == Role::Owner { Ok(true) }
        else { Err(RoleError::RoleChangeRejection) }
    }
}

// todo: remove the GroupUsersRoleFromJson struct entirely and provide the serializer and deserializer for GroupUsersRole
impl TryFrom<GroupUsersRoleFromJson> for GroupUsersRole {
    type Error = RoleError;

    fn try_from(val: GroupUsersRoleFromJson) -> Result<Self, Self::Error> {
        let iter = val.0.into_iter();

        let mut map = HashMap::new();
        for (role_str, vec) in iter {
            map.insert(Role::try_from(role_str)?, vec);
        }

        Ok(Self(map))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GroupRolePrivileges (pub HashMap<Role, Privileges>);

#[derive(Clone)]
pub struct SocketGroupRolePrivileges (pub HashMap<Role, Arc<RwLock<Privileges>>>);

impl From<GroupRolePrivileges> for SocketGroupRolePrivileges {
    fn from(val: GroupRolePrivileges) -> Self {
        SocketGroupRolePrivileges(
            val.0
                .into_iter()
                .map(|(k, v)| (k, Arc::new(RwLock::new(v))))
                .collect::<HashMap<_, _>>()
        )
    }
}

impl GroupRolePrivileges {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get_privileges(&self, role: Role) -> Option<Privileges> {
        if role == Role::Owner { return Some(Privileges::MAX) };
        Some(*self.0.get(&role)?)
    }
}

impl SocketGroupRolePrivileges {
    pub async fn get_privileges(&self, role: Role) -> Option<Privileges> {
        if role == Role::Owner { Some(Privileges::MAX) }
        else {
            Some(*self.0.get(&role)?.read().await)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct NewGroupRolePrivilegesFromJson(pub HashMap<String, Privileges>);

#[derive(Debug, PartialEq, Clone)]
pub struct NewGroupRolePrivileges(pub HashMap<Role, Privileges>);

impl NewGroupRolePrivileges {
    // todo: refactor this, I ran into borrow checker issues writing this function
    pub async fn maintain_hierarchy(&mut self, other: &SocketGroupRolePrivileges) -> Result<(), RoleError> {
        let mut role_iter = self.0.keys().copied().collect::<Vec<Role>>();
        role_iter.sort();
        for role in role_iter.iter() {
            let Some(other_role) = role.decrement() else { continue };
            let other_privileges = match self.0.get(&other_role) {
                Some(&r) => r,
                None => *other.0.get(&other_role).context("Mismatched roles")?.read().await,
            };
            let ref_mut = self.0.get_mut(&role).unwrap();
            ref_mut.cmp_with_lower(other_privileges);
        }

        role_iter.reverse();
        for role in role_iter {
            let Some(other_role) = role.increment() else { continue };
            let other_privileges = match self.0.get(&other_role) {
                Some(&r) => r,
                None => *other.0.get(&other_role).context("Mismatched roles")?.read().await,
            };
            let ref_mut = self.0.get_mut(&role).unwrap();
            ref_mut.cmp_with_higher(other_privileges);
        }

        Ok(())
    }
}

impl TryFrom<NewGroupRolePrivilegesFromJson> for NewGroupRolePrivileges {
    type Error = RoleError;

    fn try_from(val: NewGroupRolePrivilegesFromJson) -> Result<Self, Self::Error> {
        let iter = val.0.into_iter();

        let mut map = HashMap::new();
        for (role_str, vec) in iter {
            map.insert(Role::try_from(role_str)?, vec);
        }

        Ok(Self(map))
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Privileges {
    pub can_invite: CanInvite,
    pub can_send_messages: CanSendMessages,
}

impl Privileges {
    pub const MAX: Self = Self {
        can_invite: CanInvite::Yes,
        can_send_messages: CanSendMessages::Yes(0),
    };

    fn cmp_with_lower(&mut self, other: Self) {
        self.can_invite = std::cmp::max(self.can_invite, other.can_invite);
        self.can_send_messages = std::cmp::max(self.can_send_messages, other.can_send_messages);
    }

    fn cmp_with_higher(&mut self, other: Self) {
        self.can_invite = std::cmp::min(self.can_invite, other.can_invite);
        self.can_send_messages = std::cmp::min(self.can_send_messages, other.can_send_messages);
    }
}

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Debug)]
pub enum CanInvite {
    No,
    Yes,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum CanSendMessages {
    No,
    Yes(usize),
}

impl PartialOrd for CanSendMessages {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let cmp_res = match self {
            CanSendMessages::No => match other {
                CanSendMessages::No => Ordering::Equal,
                CanSendMessages::Yes(_) => Ordering::Less,
            }
            CanSendMessages::Yes(x) => match other {
                CanSendMessages::No => Ordering::Greater,
                CanSendMessages::Yes(y) => y.cmp(x),
            }
        };
        Some(cmp_res)
    }
}

impl Ord for CanSendMessages {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // the result is always Some(_)
        self.partial_cmp(other).unwrap()
    }
}
