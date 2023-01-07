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

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[sqlx(type_name = "group_user")]
pub struct GroupUser {
    pub user_id: Uuid,
    pub group_id: Uuid,
}

impl GroupUser {
    pub fn new(user_id: Uuid, group_id: Uuid) -> Self {
        Self {
            user_id,
            group_id,
        }
    }
}

impl sqlx::postgres::PgHasArrayType for GroupUser {
    fn array_type_info() -> PgTypeInfo {
        // array types in pgsql have underscores before their type name
        PgTypeInfo::with_name("_group_user")
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
