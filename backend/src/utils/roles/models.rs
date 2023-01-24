use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    hash::Hash,
};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::utils::groups::models::GroupUser;

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

// impl Role {
//     fn increment(self) -> Option<Self> {
//         match self {
//             Role::Member => Some(Role::Admin),
//             Role::Admin => Some(Role::Owner),
//             Role::Owner => None,
//         }
//     }

//     fn decrement(self) -> Option<Self> {
//         match self {
//             Role::Member => None,
//             Role::Admin => Some(Role::Member),
//             Role::Owner => Some(Role::Admin),
//         }
//     }
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GroupUsersRole {
    pub group_id: Uuid,
    pub new_roles: HashMap<Role, Vec<Uuid>>,
}

impl GroupUsersRole {
    pub fn preprocess(&mut self, role: Role, user: Uuid) -> Result<(), RoleError> {
        self.new_roles.iter_mut().for_each(|(_, vec)| {
            vec.retain(|&x| x != user);
        });
        self.new_roles.retain(|_, vec| !vec.is_empty());

        if self.verify_before_role_change(role)? {
            self.new_roles
                .entry(Role::Admin)
                .and_modify(|vec| vec.push(user))
                .or_insert(vec![user]);
        };

        Ok(())
    }

    fn verify_before_role_change(&mut self, role: Role) -> Result<bool, RoleError> {
        if role == Role::Member {
            return Err(RoleError::RoleChangeRejection);
        };
        if self.new_roles.get(&Role::Owner).is_none() {
            return Ok(false);
        };
        let new_owners = self.new_roles.get(&Role::Owner).unwrap();
        if new_owners.is_empty() {
            Ok(false)
        } else if new_owners.len() == 1 && role == Role::Owner {
            Ok(true)
        } else {
            Err(RoleError::RoleChangeRejection)
        }
    }
}

impl<const N: usize> From<(Uuid, [(Role, Vec<Uuid>); N])> for GroupUsersRole {
    fn from(val: (Uuid, [(Role, Vec<Uuid>); N])) -> Self {
        Self {
            group_id: val.0,
            new_roles: HashMap::from(val.1),
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

// impl Privileges {
//     fn cmp_with_lower(&mut self, other: &Self) -> Result<(), RoleError> {
//         for privilege_type in privilege_type_iter {
//             let self_privilege = self
//                 .0
//                 .get_mut(&privilege_type)
//                 .context("Mismatched privileges")?;
//             let other_privilege = other
//                 .0
//                 .get(&privilege_type)
//                 .context("Mismatched privileges")?;
//             self_privilege.cmp_with_lower(other_privilege)?;
//         }
//         Ok(())
//     }

//     fn cmp_with_higher(&mut self, other: &Self) -> Result<(), RoleError> {
//         let privilege_type_iter = self.0.keys().copied().collect::<Vec<PrivilegeType>>();

//         for privilege_type in privilege_type_iter {
//             let self_privilege = self
//                 .0
//                 .get_mut(&privilege_type)
//                 .context("Mismatched privileges")?;
//             let other_privilege = other
//                 .0
//                 .get(&privilege_type)
//                 .context("Mismatched privileges")?;
//             self_privilege.cmp_with_higher(other_privilege)?;
//         }
//         Ok(())
//     }
// }

// impl Privilege {
//     fn cmp_with_lower(&mut self, other: &Self) -> Result<(), RoleError> {
//         if (&*self)
//             .partial_cmp(other)
//             .context("Mismatched privileges")?
//             == Ordering::Less
//         {
//             self.try_set(other)?;
//         };
//         Ok(())
//     }

//     fn cmp_with_higher(&mut self, other: &Self) -> Result<(), RoleError> {
//         if (&*self)
//             .partial_cmp(other)
//             .context("Mismatched privileges")?
//             == Ordering::Greater
//         {
//             self.try_set(other)?;
//         };
//         Ok(())
//     }

//     fn try_set(&mut self, other: &Self) -> Result<(), RoleError> {
//         match self {
//             Privilege::CanInvite(x) => match other {
//                 Privilege::CanInvite(y) => Ok(*x = *y),
//                 _ => Err(RoleError::Unexpected(anyhow!("Mismatched privileges"))),
//             },
//             Privilege::CanSendMessages(x) => match other {
//                 Privilege::CanSendMessages(y) => Ok(*x = *y),
//                 _ => Err(RoleError::Unexpected(anyhow!("Mismatched privileges"))),
//             },
//         }
//     }
// }

// impl PartialOrd for Privilege {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         match self {
//             Privilege::CanInvite(x) => match other {
//                 Privilege::CanInvite(y) => x.partial_cmp(y),
//                 _ => None,
//             },
//             Privilege::CanSendMessages(x) => match other {
//                 Privilege::CanSendMessages(y) => x.partial_cmp(y),
//                 _ => None,
//             },
//         }
//     }
// }

#[derive(Deserialize, Serialize, Debug)]
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

// impl PrivilegeChangeData {
//     pub async fn maintain_hierarchy(
//         &mut self,
//         other: &SocketGroupRolePrivileges,
//     ) -> Result<(), RoleError> {
//         if let Some(other_role) = self.role.decrement() {
//             if let Some(other_privileges_lock) = other.0.get(&other_role) {
//                 let other_privileges = other_privileges_lock.read().await;
//                 if let Some(privilege) = other_privileges.0.get(&self.privilege) {
//                     let ref_mut = &mut self.value;
//                     ref_mut.cmp_with_lower(privilege)?;
//                 }
//             }
//         };

//         match self.role.increment() {
//             Some(other_role) if other_role != Role::Owner => {
//                 if let Some(other_privileges_lock) = other.0.get(&other_role) {
//                     let other_privileges = other_privileges_lock.read().await;
//                     if let Some(privilege) = other_privileges.0.get(&self.privilege) {
//                         let ref_mut = &mut self.value;
//                         ref_mut.cmp_with_higher(privilege)?;
//                     }
//                 }
//             }
//             _ => (),
//         };

//         Ok(())
//     }
// }

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

#[derive(Serialize, Deserialize)]
pub struct BulkChangePrivileges(pub Vec<PrivilegeChangeData>);

#[derive(Serialize, Deserialize)]
pub struct BulkRoleChangeData(pub Vec<UserRoleChangeData>);

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
