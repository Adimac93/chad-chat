use std::{collections::{HashSet, HashMap}, cmp::Ordering, hash::Hash, mem::discriminant};

use serde::{Serialize, Deserialize};
use uuid::Uuid;

use super::models::Role;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Privileges(pub HashSet<Privilege>);

impl Privileges {
    pub fn max() -> Self {
        Self(HashSet::from([
            Privilege::CanInvite(CanInvite::Yes),
            Privilege::CanSendMessages(CanSendMessages::Yes(0)),
        ]))
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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "privilege_type", rename_all = "snake_case")]
pub enum PrivilegeType {
    CanInvite,
    CanSendMessages,
}

#[derive(Serialize, Deserialize)]
pub struct QueryPrivileges(pub HashMap<PrivilegeType, Privilege>);

impl From<Privileges> for QueryPrivileges {
    fn from(val: Privileges) -> Self {
        QueryPrivileges(val.0.into_iter().map(|x| {
            (match x {
                Privilege::CanInvite(_) => PrivilegeType::CanInvite,
                Privilege::CanSendMessages(_) => PrivilegeType::CanSendMessages,
            }, x)
        }).collect::<HashMap<_, _>>())
    }
}

impl From<QueryPrivileges> for Privileges {
    fn from(val: QueryPrivileges) -> Self {
        Privileges(val.0.into_iter().map(|(_, x)| x).collect::<HashSet<_>>())
    }
}
