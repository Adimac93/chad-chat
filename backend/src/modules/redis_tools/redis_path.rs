use std::fmt::Display;

use uuid::Uuid;

use crate::utils::roles::models::Role;

pub struct RedisRoot;

impl RedisRoot {
    pub fn group(self, id: Uuid) -> RedisGroup {
        RedisGroup(id)
    }
}

impl Display for RedisRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

pub struct RedisGroup(Uuid);

impl RedisGroup {
    pub fn user(self, id: Uuid) -> RedisGroupUser {
        RedisGroupUser(self.0, id)
    }

    pub fn role(self, role: Role) -> RedisGroupRole {
        RedisGroupRole(self.0, role)
    }
}

impl Display for RedisGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "group:{}", self.0)
    }
}

pub struct RedisGroupUser(Uuid, Uuid);

impl RedisGroupUser {
    pub fn role(self) -> RedisGroupUserRole {
        RedisGroupUserRole(self.0, self.1)
    }
}

impl Display for RedisGroupUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "group:{}:user:{}", self.0, self.1)
    }
}

pub struct RedisGroupUserRole(Uuid, Uuid);

impl Display for RedisGroupUserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "group:{}:user:{}:role", self.0, self.1)
    }
}

pub struct RedisGroupRole(Uuid, Role);

impl Display for RedisGroupRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "group:{}:role:{}", self.0, self.1)
    }
}
