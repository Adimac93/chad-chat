use std::{cmp::Ordering, collections::{HashMap, HashSet}};

use serde::{Serialize, Deserialize};

use crate::utils::groups::models::GroupUser;

use super::errors::RoleError;

#[derive(sqlx::Type, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
pub enum Role {
    Member,
    Admin,
    Owner,
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Copy)]
pub struct GroupRolePrivileges {
    pub admin: Privileges,
    pub member: Privileges,
}

impl GroupRolePrivileges {
    pub fn maintain_hierarchy(&mut self) {
        self.admin.can_invite = std::cmp::max(self.admin.can_invite, self.member.can_invite);
        self.admin.can_send_messages = std::cmp::max(self.admin.can_send_messages, self.member.can_send_messages);
    }

    // todo: there is probably a better way to signal changed privileges
    pub fn compare(&self, other: &Self) -> ChangedPrivilegeSet {
        let mut res = HashSet::new();
        if self.admin != other.admin { res.insert(Role::Admin); }
        if self.member != other.member { res.insert(Role::Member); }

        ChangedPrivilegeSet(res)
    }

    pub fn get_privileges(&self, role: Role) -> Privileges {
        match role {
            Role::Owner => Privileges::MAX,
            Role::Admin => self.admin,
            Role::Member => self.member,
        }
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
