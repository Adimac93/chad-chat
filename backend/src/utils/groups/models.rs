use serde::{Deserialize, Serialize};
use typeshare::typeshare;
use uuid::Uuid;

#[typeshare]
#[derive(Serialize, Deserialize)]
pub struct NewGroup {
    pub name: String,
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
}

#[typeshare]
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

#[typeshare]
#[derive(Deserialize)]
pub struct NewGroupInvitation {
    pub group_id: Uuid,
}

#[typeshare]
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct GroupInfo {
    pub name: String,
    pub members: i32,
}
