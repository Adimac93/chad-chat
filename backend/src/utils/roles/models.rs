use redis::{FromRedisValue, RedisError, ErrorKind};
use serde::{Deserialize, Serialize};
use std::{hash::Hash, fmt::Display, str::from_utf8};
use typeshare::typeshare;
use uuid::Uuid;

use crate::errors::TryFromStrError;

use super::privileges::Privilege;

#[typeshare]
#[derive(Serialize)]
pub struct ReceiveRoleOutput {
    pub role: Role,
}

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
    pub value: Role,
}

impl UserRoleChangeInput {
    pub fn new(group_id: Uuid, value: Role) -> Self {
        Self {
            group_id,
            value,
        }
    }
}
