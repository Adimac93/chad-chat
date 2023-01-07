pub mod errors;
pub mod models;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool, types::JsonValue, query_as};
use uuid::Uuid;

use self::errors::RoleError;

use super::groups::models::GroupUser;

pub async fn get_group_role_privileges(pool: &PgPool, group_id: Uuid) -> Result<GroupRolePrivileges, RoleError> {
    let mut query_res = query_as!(
        QueryPrivileges,
        r#"
            select roles.privileges from
                group_roles inner join roles on group_roles.role_id = roles.id
                where group_roles.group_id = $1
                and group_roles.role_type in ('member', 'admin')
                order by group_roles.role_type
        "#,
        group_id
    )
    .fetch_all(pool)
    .await
    .context("Failed to update group role privileges")?
    .into_iter();

    let admin_privileges: Privileges = serde_json::from_value(query_res.next().context("Database has insufficient privilege data")?.privileges)
        .context("Failed to deserialize privileges from JSON")?;
    let member_privileges: Privileges = serde_json::from_value(query_res.next().context("Database has insufficient privilege data")?.privileges)
        .context("Failed to deserialize privileges from JSON")?;

    Ok(GroupRolePrivileges {
        admin: admin_privileges,
        member: member_privileges,
    })
}

pub async fn set_group_role_privileges(pool: &PgPool, group_id: Uuid, mut new_privileges: GroupRolePrivileges) -> Result<GroupRolePrivileges, RoleError> {
    new_privileges.maintain_hierarchy();

    let mut transaction = pool.begin().await.context("Failed to begin transaction")?;

    for (role, privileges) in [(Role::Admin, &new_privileges.admin), (Role::Member, &new_privileges.member)] {
        // rollbacks automatically on error
        let _res = query!(
            r#"
                update roles
                    set privileges = $1
                    where roles.id = (
                        select role_id
                            from group_roles
                            where group_roles.role_type = $2
                            and group_roles.group_id = $3
                    )
            "#,
            &serde_json::to_value(&privileges).context("Failed to convert privileges to JSON")?,
            &role as &Role,
            &group_id,
        )
        .execute(&mut transaction)
        .await
        .context("Failed to update group role privileges")?;
    }

    transaction.commit().await.context("Failed to commit transaction")?;

    Ok(new_privileges)
}

pub async fn set_group_users_role(pool: &PgPool, roles: GroupUsersRole) -> Result<(), RoleError> {
    let mut transaction = pool.begin().await.context("Failed to begin transaction")?;
    
    for (role, users) in roles.0 {

        // rollbacks automatically on error
        let _res = query!(
            r#"
                update group_users
                set role_id = (
                    select role_id
                        from group_roles
                        where group_roles.group_id = group_users.group_id
                        and group_roles.role_type = $1
                )
                where (user_id, group_id) = any($2)
            "#,
            role as Role,
            users.0 as Vec<GroupUser>,
        )
        .execute(&mut transaction)
        .await
        .context("Failed to update user roles")?;
    }

    transaction.commit().await.context("Failed to commit transaction")?;

    Ok(())
}

pub async fn get_user_role(pool: &PgPool, user_id: Uuid, group_id: Uuid) -> Result<Role, RoleError> {
    let res = query!(
        r#"
            select
                group_roles.role_type as "role: Role"
                from group_users
                inner join
                    roles inner join group_roles on roles.id = group_roles.role_id
                on group_users.role_id = roles.id
                where group_users.user_id = $1
                and group_users.group_id = $2
        "#,
        user_id,
        group_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to fetch user role")?;

    Ok(res.role)
}

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

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq)]
#[sqlx(transparent)]
pub struct GroupUsersVec(pub Vec<GroupUser>);

#[derive(Deserialize, Serialize, Clone)]
pub struct GroupUsersRoleFromJson(pub HashMap<String, GroupUsersVec>);

#[derive(Clone, Debug, PartialEq)]
pub struct GroupUsersRole(pub HashMap<Role, GroupUsersVec>);

pub struct ChangedPrivilegeSet(pub HashSet<Role>);

impl GroupUsersRole {
    pub fn verify_before_role_change(&mut self, role: Role, user: GroupUser) -> Result<(), RoleError> {
        match role {
            Role::Owner | Role::Admin => {
                self.0.iter_mut().for_each(|(_, vec)| {
                    vec.0.retain(|x| x.user_id != user.user_id);
                });
                self.0.retain(|_, vec| {!vec.0.is_empty()});

                if let Some(new_owners) = self.0.get(&Role::Owner) {
                    match new_owners.0.len() {
                        0 => Ok(()),
                        1 => if role == Role::Owner {
                            self.0.entry(Role::Admin)
                                .and_modify(|vec| vec.0.push(user))
                                .or_insert(GroupUsersVec(vec![user]));
                            Ok(())
                        } else { Err(RoleError::RoleChangeRejection) }
                        _ => Err(RoleError::RoleChangeRejection),
                    }
                } else { Ok(()) }
            },
            Role::Member => Err(RoleError::RoleChangeRejection),
        }
    }
}

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

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct GroupRolePrivileges {
    pub admin: Privileges,
    pub member: Privileges,
}

impl GroupRolePrivileges {
    pub fn maintain_hierarchy(&mut self) {
        self.admin.can_invite = std::cmp::max(self.admin.can_invite, self.member.can_invite);
        self.admin.can_send_messages = std::cmp::max(self.admin.can_send_messages, self.member.can_send_messages);
    }

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

pub struct QueryPrivileges{
    privileges: JsonValue,
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
                CanSendMessages::Yes(y) => match x.cmp(y) {
                    // the more slow chat seconds, the less beneficial the privilege
                    Ordering::Less => Ordering::Greater,
                    Ordering::Equal => Ordering::Equal,
                    Ordering::Greater => Ordering::Less,
                }
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

/*
group users role
(asdf-asdf-asdf-asdf, asdf-asdf-asdf-asdf), member
(asdf-asdf-sadf-asdf, asdf-asdf-sadf-asdf), admin
*/

/*
update role privilege values given the group id and role type

group role privileges
(asdf-asdf-asdf-asdf, asdf-asdf-asdf-asdf), member
(asdf-asdf-sadf-asdf, asdf-asdf-sadf-asdf), admin
(asdf-asdf-sadf-fdsa, asdf-asdf-sadf-fdsa), owner

select role_id from group_id where group_id = .. and role_type = ..

update roles inner join on roles.id = group_roles.role_id
    set roles.privileges = $1
    where group_roles.role_id = $2
    and group_roles.role_type = $3
*/
