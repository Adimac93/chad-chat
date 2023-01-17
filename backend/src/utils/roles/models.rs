use std::{cmp::Ordering, collections::{HashMap, HashSet}, sync::Arc};

use anyhow::{Context, anyhow};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::utils::groups::models::GroupUser;

use super::errors::RoleError;

#[derive(sqlx::Type, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
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

// impl TryFrom<String> for Role {
//     type Error = RoleError;

//     fn try_from(val: String) -> Result<Self, Self::Error> {
//         match &*val {
//             "owner" => Ok(Role::Owner),
//             "admin" => Ok(Role::Admin),
//             "member" => Ok(Role::Member),
//             _ => Err(RoleError::RoleParseError),
//         }
//     }
// }

// #[derive(Deserialize, Serialize, Clone)]
// pub struct GroupUsersRoleFromJson(pub HashMap<String, Vec<GroupUser>>);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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

// impl TryFrom<GroupUsersRoleFromJson> for GroupUsersRole {
//     type Error = RoleError;

//     fn try_from(val: GroupUsersRoleFromJson) -> Result<Self, Self::Error> {
//         let iter = val.0.into_iter();

//         let mut map = HashMap::new();
//         for (role_str, vec) in iter {
//             map.insert(Role::try_from(role_str)?, vec);
//         }

//         Ok(Self(map))
//     }
// }

#[derive(Debug, PartialEq, Clone)]
pub struct GroupRolePrivileges (pub HashMap<Role, Privileges>);

#[derive(Clone)]
pub struct SocketGroupRolePrivileges (pub HashMap<Role, Arc<RwLock<Privileges>>>);

impl SocketGroupRolePrivileges {
    pub async fn get_privileges(&self, role: Role) -> Option<Privileges> {
        if role == Role::Owner { Some(Privileges::max()) }
        else { Some(self.0.get(&role)?.read().await.clone()) }
    }
}

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
        if role == Role::Owner { return Some(Privileges::max()) };
        Some(self.0.get(&role)?.clone())
    }
}

// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
// pub struct BulkNewGroupRolePrivilegesFromJson(pub HashMap<String, Privileges>);

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct BulkNewGroupRolePrivileges(pub HashMap<Role, Privileges>);

impl BulkNewGroupRolePrivileges {
    // todo: refactor this, I ran into borrow checker issues writing this function
    pub async fn maintain_hierarchy(&mut self, other: &SocketGroupRolePrivileges) -> Result<(), RoleError> {
        let mut role_iter = self.0.keys().copied().collect::<Vec<Role>>();
        role_iter.sort();
        for role in role_iter.iter() {
            let Some(other_role) = role.decrement() else { continue };
            let mut ref_privilege = self.0.remove(&role).unwrap();
            let arc_rwlock;
            let other_privileges = match self.0.get(&other_role) {
                Some(r) => r,
                None => match other.0.get(&other_role) {
                    Some(x) => {
                        arc_rwlock = x.read().await;
                        &arc_rwlock
                    },
                    None => {
                        self.0.insert(*role, ref_privilege);
                        return Err(RoleError::Unexpected(anyhow!("Mismatched privileges")))
                    }
                }
            };
            
            let ref_mut = &mut ref_privilege;
            ref_mut.cmp_with_lower(other_privileges)?;
            self.0.insert(*role, ref_privilege);
        }

        role_iter.reverse();
        for role in role_iter.iter() {
            let Some(other_role) = role.increment() else { continue };
            if other_role == Role::Owner { continue };
            let mut ref_privilege = self.0.remove(&role).unwrap();
            let arc_rwlock;
            let other_privileges = match self.0.get(&other_role) {
                Some(r) => r,
                None => match other.0.get(&other_role) {
                    Some(x) => {
                        arc_rwlock = x.read().await;
                        &arc_rwlock
                    },
                    None => {
                        self.0.insert(*role, ref_privilege);
                        return Err(RoleError::Unexpected(anyhow!("Mismatched privileges")))
                    }
                }
            };
            
            let ref_mut = &mut ref_privilege;
            ref_mut.cmp_with_higher(other_privileges)?;
            self.0.insert(*role, ref_privilege);
        }

        Ok(())
    }
}

// impl TryFrom<BulkNewGroupRolePrivilegesFromJson> for BulkNewGroupRolePrivileges {
//     type Error = RoleError;

//     fn try_from(val: BulkNewGroupRolePrivilegesFromJson) -> Result<Self, Self::Error> {
//         let iter = val.0.into_iter();

//         let mut map = HashMap::new();
//         for (role_str, vec) in iter {
//             map.insert(Role::try_from(role_str)?, vec);
//         }

//         Ok(Self(map))
//     }
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Privileges(pub HashMap<PrivilegeType, Privilege>);

impl Privileges {
    pub fn max() -> Self {
        Self(HashMap::from([
            (PrivilegeType::CanInvite, Privilege::CanInvite(CanInvite::Yes)),
            (PrivilegeType::CanSendMessages, Privilege::CanSendMessages(CanSendMessages::Yes(0))),
        ]))
    }

    fn cmp_with_lower(&mut self, other: &Self) -> Result<(), RoleError> {
        let privilege_type_iter = self.0.keys().copied().collect::<Vec<PrivilegeType>>();

        for privilege_type in privilege_type_iter {
            let self_privilege = self.0.get_mut(&privilege_type).context("Mismatched privileges")?;
            let other_privilege = other.0.get(&privilege_type).context("Mismatched privileges")?;
            self_privilege.cmp_with_lower(other_privilege)?;
        }
        Ok(())
    }

    fn cmp_with_higher(&mut self, other: &Self) -> Result<(), RoleError> {
        let privilege_type_iter = self.0.keys().copied().collect::<Vec<PrivilegeType>>();

        for privilege_type in privilege_type_iter {
            let self_privilege = self.0.get_mut(&privilege_type).context("Mismatched privileges")?;
            let other_privilege = other.0.get(&privilege_type).context("Mismatched privileges")?;
            self_privilege.cmp_with_higher(other_privilege)?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum CanInvite {
    No,
    Yes,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "privilege_type", rename_all = "snake_case")]
pub enum PrivilegeType {
    CanInvite,
    CanSendMessages,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
 pub enum Privilege {
    CanInvite(CanInvite),
    CanSendMessages(CanSendMessages),
 }

 impl Privilege {
    fn cmp_with_lower(&mut self, other: &Self) -> Result<(), RoleError> {
        if (&*self).partial_cmp(other).context("Mismatched privileges")? == Ordering::Less {
            self.try_set(other)?;
        };
        Ok(())
    }

    fn cmp_with_higher(&mut self, other: &Self) -> Result<(), RoleError> {
        if (&*self).partial_cmp(other).context("Mismatched privileges")? == Ordering::Greater {
            self.try_set(other)?;
        };
        Ok(())
    }

    fn try_set(&mut self, other: &Self) -> Result<(), RoleError> {
        match self {
            Privilege::CanInvite(x) => match other {
                Privilege::CanInvite(y) => Ok(*x = *y),
                _ => Err(RoleError::Unexpected(anyhow!("Mismatched privileges"))),
            },
            Privilege::CanSendMessages(x) => match other {
                Privilege::CanSendMessages(y) => Ok(*x = *y),
                _ => Err(RoleError::Unexpected(anyhow!("Mismatched privileges"))),
            },
        }
    }
 }

 impl PartialOrd for Privilege {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            Privilege::CanInvite(x) => match other {
                Privilege::CanInvite(y) => x.partial_cmp(y),
                _ => None,
            }
            Privilege::CanSendMessages(x) => match other {
                Privilege::CanSendMessages(y) => x.partial_cmp(y),
                _ => None,
            }
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PrivilegeChangeData {
    pub group_id: Uuid,
    pub role: Role,
    pub privilege: PrivilegeType,
    pub value: Privilege,
}

impl PrivilegeChangeData {
    pub async fn maintain_hierarchy(&mut self, other: &SocketGroupRolePrivileges) -> Result<(), RoleError> {
        if let Some(other_role) = self.role.decrement() {
            if let Some(other_privileges_lock) = other.0.get(&other_role) {
                let other_privileges = other_privileges_lock.read().await;
                if let Some(privilege) = other_privileges.0.get(&self.privilege) {
                    let ref_mut = &mut self.value;
                    ref_mut.cmp_with_lower(privilege)?;
                }
            }
        };
        
        match self.role.increment() {
            Some(other_role) if other_role != Role::Owner => {
                if let Some(other_privileges_lock) = other.0.get(&other_role) {
                    let other_privileges = other_privileges_lock.read().await;
                    if let Some(privilege) = other_privileges.0.get(&self.privilege) {
                        let ref_mut = &mut self.value;
                        ref_mut.cmp_with_higher(privilege)?;
                    }
                }
            }
            _ => (),
        };
        
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserRoleChangeData {
    pub group_id: Uuid,
    pub user_id: Uuid,
    pub value: Role,
}
