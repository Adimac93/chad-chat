use std::{cmp::Ordering, collections::HashSet, hash::Hash, mem::discriminant};

use anyhow::anyhow;
use axum::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{query, Acquire, Postgres};

use crate::utils::roles::models::Role;

use crate::errors::AppError;
use super::models::PrivilegeChangeInput;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Privileges(pub HashSet<Privilege>);

impl Default for Privileges {
    fn default() -> Self {
        Self::new()
    }
}

impl Privileges {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn max() -> Self {
        Self::from([
            Privilege::CanInvite(CanInvite::Yes),
            Privilege::CanSendMessages(CanSendMessages::Yes(0)),
        ])
    }
}

impl<const N: usize> From<[Privilege; N]> for Privileges {
    fn from(val: [Privilege; N]) -> Self {
        Self(HashSet::from(val))
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
            },
            CanSendMessages::Yes(x) => match other {
                CanSendMessages::No => Ordering::Greater,
                CanSendMessages::Yes(y) => y.cmp(x),
            },
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum Privilege {
    CanInvite(CanInvite),
    CanSendMessages(CanSendMessages),
}

impl PartialEq for Privilege {
    fn eq(&self, other: &Self) -> bool {
        discriminant(self) == discriminant(other)
    }
}

impl Hash for Privilege {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

#[async_trait]
pub trait QueryPrivilege<'c> {
    async fn set_privilege(
        &self,
        conn: impl Acquire<'c, Database = Postgres> + std::marker::Send,
        data: &PrivilegeChangeInput,
    ) -> sqlx::Result<()>;
}

#[async_trait]
impl<'c> QueryPrivilege<'c> for CanInvite {
    async fn set_privilege(
        &self,
        conn: impl Acquire<'c, Database = Postgres> + std::marker::Send,
        data: &PrivilegeChangeInput,
    ) -> sqlx::Result<()> {
        let mut transaction = conn.begin().await?;

        let val = match self {
            CanInvite::Yes => true,
            CanInvite::No => false,
        };

        let _res = query!(
            r#"
                UPDATE roles
                    SET can_invite = $1
                    from group_roles
                    WHERE group_roles.group_id = $2
                    AND group_roles.role_type = $3
            "#,
            val,
            data.group_id,
            data.role as Role,
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(())
    }
}

impl From<bool> for CanInvite {
    fn from(val: bool) -> Self {
        match val {
            true => CanInvite::Yes,
            false => CanInvite::No,
        }
    }
}

#[async_trait]
impl<'c> QueryPrivilege<'c> for CanSendMessages {
    async fn set_privilege(
        &self,
        conn: impl Acquire<'c, Database = Postgres> + std::marker::Send,
        data: &PrivilegeChangeInput,
    ) -> sqlx::Result<()> {
        let mut transaction = conn.begin().await?;

        let val = match self {
            CanSendMessages::Yes(x) => *x as i32,
            CanSendMessages::No => -1,
        };

        let _res = query!(
            r#"
                UPDATE roles
                    SET can_send_messages = $1
                    from group_roles
                    WHERE group_roles.group_id = $2
                    AND group_roles.role_type = $3
            "#,
            val,
            data.group_id,
            data.role as Role,
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(())
    }
}

impl TryFrom<i32> for CanSendMessages {
    type Error = AppError;

    fn try_from(val: i32) -> Result<Self, Self::Error> {
        match val {
            ..=-2 => Err(AppError::Unexpected(anyhow!("Failed to interpret privileges"))),
            -1 => Ok(CanSendMessages::No),
            // an extra assertion is used to ensure that it won't panic (not necessary)
            0..=i32::MAX => Ok(CanSendMessages::Yes(val as usize)),
        }
    }
}
