use serde::{Deserialize, Serialize};
use sqlx::postgres::PgTypeInfo;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct NewGroup {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct GroupUser {
    pub user_id: Uuid,
    pub group_id: Uuid,
}

impl GroupUser {
    pub fn new(user_id: Uuid, group_id: Uuid) -> Self {
        Self { user_id, group_id }
    }
}

#[derive(Deserialize)]
pub struct NewGroupInvitation {
    pub group_id: Uuid,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct GroupInfo {
    pub name: String,
    pub members: i64,
}
