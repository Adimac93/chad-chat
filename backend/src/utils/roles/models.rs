use redis::{FromRedisValue, RedisError, ErrorKind};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash, fmt::Display, str::from_utf8};
use typeshare::typeshare;
use uuid::Uuid;

use crate::errors::TryFromStrError;

#[typeshare]
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

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = match self {
            Self::Admin => "admin",
            Self::Member => "member",
            Self::Owner => "owner",
        };

        write!(f, "{res}")
    }
}

impl TryFrom<&str> for Role {
    type Error = TryFromStrError;

    fn try_from(val: &str) -> Result<Self, Self::Error> {
        match val {
            "admin" => Ok(Role::Admin),
            "member" => Ok(Role::Member),
            "owner" => Ok(Role::Owner),
            _ => Err(TryFromStrError::new("expected \"admin\", \"member\" or \"owner\""))
        }
    }
}

impl FromRedisValue for Role {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        match v {
            redis::Value::Data(d) => {
                let s = from_utf8(d).map_err(|_| RedisError::from((ErrorKind::TypeError, "Expected UTF-8 string")))?;
                // TODO: use the error returned by the TryFromStrError
                Role::try_from(s).map_err(|_| RedisError::from((ErrorKind::ResponseError, "expected \"admin\", \"member\" or \"owner\"")))
            },
            _ => Err(RedisError::from((ErrorKind::TypeError, "Expected UTF-8 string"))),
        }
    }
}

#[typeshare]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(tag = "type", content = "content")]
pub enum Privilege {
    CanInvite(bool),
    CanSendMessages(bool),
}

impl Privilege {
    /// Interprets the privilege in terms of updated bits. The first number of the result represents the decimal representation of bits,
    /// shifted by the necessary amount. The second result consists of ones at the places concerning a given privilege.
    pub fn to_bits(self) -> (u8, u8) {
        match self {
            Self::CanInvite(v) => if v {
                (1, 0)
            } else {
                (0, 0)
            },
            Self::CanSendMessages(v) => if v {
                (1 << 1, 1 << 1)
            } else {
                (0 << 1, 1 << 1)
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct PrivilegeChangeInput {
    pub group_id: Uuid,
    pub role: Role,
    pub value: Privilege,
}

impl PrivilegeChangeInput {
    pub fn new(group_id: Uuid, role: Role, value: Privilege) -> Self {
        Self {
            group_id,
            role,
            value,
        }
    }
}

#[typeshare]
#[derive(Serialize, Deserialize)]
pub struct UserRoleChangeInput {
    pub group_id: Uuid,
    pub user_id: Uuid,
    pub value: Role,
}

impl UserRoleChangeInput {
    pub fn new(group_id: Uuid, user_id: Uuid, value: Role) -> Self {
        Self {
            group_id,
            user_id,
            value,
        }
    }
}

#[typeshare]
#[derive(Serialize)]
pub struct GroupPrivileges {
    pub privileges: HashMap<Role, u8>,
}

pub struct UserPrivileges {
    pub privileges: u8,
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
