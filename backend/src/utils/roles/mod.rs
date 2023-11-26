pub mod models;
pub mod privileges;

use sqlx::{Acquire, PgPool, Postgres};
use uuid::Uuid;

use crate::{errors::AppError, state::RdPool};
use self::models::{
        PrivilegeChangeInput, Role,
        UserRoleChangeInput, GroupPrivileges, UserPrivileges,
    };

pub async fn set_privileges<'c>(
    conn: &RdPool,
    data: &PrivilegeChangeInput,
) -> Result<(), AppError> {
    todo!()
}

pub async fn get_all_privileges(
    pool: &RdPool,
    group_id: Uuid,
) -> Result<GroupPrivileges, AppError> {
    todo!()
}

pub async fn set_role<'c>(
    conn: &RdPool,
    data: &UserRoleChangeInput,
) -> Result<(), AppError> {
    todo!()
}

pub async fn get_user_role(
    pool: &RdPool,
    user_id: &Uuid,
    group_id: &Uuid,
) -> Result<Role, AppError> {
    todo!()
}

pub async fn get_user_privileges(
    pool: &RdPool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<UserPrivileges, AppError> {
    todo!()
}
