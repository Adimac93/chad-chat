use std::collections::HashMap;

use anyhow::anyhow;
use redis::{FromRedisValue, RedisError, ErrorKind};
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

use crate::errors::AppError;

use super::{models::Role, ROLES_COUNT};

#[typeshare]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(tag = "type", content = "content")]
pub enum Privilege {
    CanInvite(CanInvite),
    CanSendMessages(CanSendMessages),
}

pub trait PrivilegeBits: Into<u8> + TryFrom<u8, Error = String> {
    const BIT_AMOUNT: u32;
    const BIT_OFFSET: u32;
    
    fn to_bits(self) -> (u8, u8) {
        (self.into() << Self::BIT_OFFSET, (2u8.pow(Self::BIT_AMOUNT) - 1) << Self::BIT_OFFSET)
    }
}

#[typeshare]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum CanInvite {
    Yes,
    No,
}

impl From<CanInvite> for u8 {
    fn from(val: CanInvite) -> Self {
        match val {
            CanInvite::No => 0,
            CanInvite::Yes => 1,
        }
    }
}

impl TryFrom<u8> for CanInvite {
    type Error = String;
    
    fn try_from(val: u8) -> Result<Self, Self::Error> {
        match val {
            0 => Ok(CanInvite::No),
            1 => Ok(CanInvite::Yes),
            _ => Err("expected number from 0 to 1".to_string()),
        }
    }
}

impl PrivilegeBits for CanInvite {
    const BIT_AMOUNT: u32 = 1;
    const BIT_OFFSET: u32 = 0;
}

#[typeshare]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(tag = "type", content = "content")]
pub enum CanSendMessages {
    Yes,
    SlowChat(SlowChat),
    No,
}

#[typeshare]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum SlowChat {
    OneSec,
    FiveSecs,
    ThirtySecs,
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
}

impl From<CanSendMessages> for u8 {
    fn from(val: CanSendMessages) -> Self {
        match val {
            CanSendMessages::No => 0,
            CanSendMessages::Yes => 1,
            CanSendMessages::SlowChat(SlowChat::OneSec) => 2,
            CanSendMessages::SlowChat(SlowChat::FiveSecs) => 3,
            CanSendMessages::SlowChat(SlowChat::ThirtySecs) => 4,
            CanSendMessages::SlowChat(SlowChat::OneMinute) => 5,
            CanSendMessages::SlowChat(SlowChat::FiveMinutes) => 6,
            CanSendMessages::SlowChat(SlowChat::FifteenMinutes) => 7,
        }
    }
}

impl TryFrom<u8> for CanSendMessages {
    type Error = String;
    
    fn try_from(val: u8) -> Result<Self, Self::Error> {
        match val {
            0 => Ok(CanSendMessages::No),
            1 => Ok(CanSendMessages::Yes),
            2 => Ok(CanSendMessages::SlowChat(SlowChat::OneSec)),
            3 => Ok(CanSendMessages::SlowChat(SlowChat::FiveSecs)),
            4 => Ok(CanSendMessages::SlowChat(SlowChat::ThirtySecs)),
            5 => Ok(CanSendMessages::SlowChat(SlowChat::OneMinute)),
            6 => Ok(CanSendMessages::SlowChat(SlowChat::FiveMinutes)),
            7 => Ok(CanSendMessages::SlowChat(SlowChat::FifteenMinutes)),
            _ => Err("expected number from 0 to 7".to_string()),
        }
    }
}

impl PrivilegeBits for CanSendMessages {
    const BIT_AMOUNT: u32 = 3;
    const BIT_OFFSET: u32 = 1;
}

impl Privilege {
    /// Interprets the privilege in terms of updated bits. The first number of the result represents target state after the update.
    /// The second number marks bits to be updated.
    pub fn to_bits(self) -> (u8, u8) {
        match self {
            Self::CanInvite(v) => v.to_bits(),
            Self::CanSendMessages(v) => v.to_bits(),
        }
    }
}

#[typeshare]
#[derive(Serialize, Clone)]
pub struct GroupPrivileges {
    pub privileges: HashMap<Role, u8>,
}

impl FromRedisValue for GroupPrivileges {
    fn from_redis_value(val: &redis::Value) -> redis::RedisResult<Self> {
        match val {
            redis::Value::Bulk(privileges) => {
                if privileges.len() != ROLES_COUNT {
                    return Err(RedisError::from((ErrorKind::ResponseError, "not enough roles")));
                }
        
                let privileges = privileges.into_iter().map(|x| u8::from_redis_value(&x)).collect::<Result<Vec<u8>, RedisError>>()?;
        
                Ok(GroupPrivileges {
                    privileges: HashMap::from([
                        (Role::Owner, privileges[0]),
                        (Role::Admin, privileges[1]),
                        (Role::Member, privileges[2]),
                    ]),
                })
            },
            _ => Err(RedisError::from((ErrorKind::TypeError, "expected \"bulk\" redis value"))),
        }
    }
}

#[derive(Clone, Copy)]
pub struct PrivilegesNumber {
    inner: u8,
}

impl PrivilegesNumber {
    pub fn new(inner: u8) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> u8 {
        self.inner
    }

    pub fn update_with(self, privilege: Privilege) -> Self {
        let (target_bits, updated_bits) = privilege.to_bits();
        Self::new(((self.inner ^ target_bits) & updated_bits) ^ self.inner)
    }

    pub fn get_privilege<T: PrivilegeBits>(self) -> Result<T, AppError> {
        let res = T::try_from((self.inner() >> T::BIT_OFFSET) & (2u8.pow(T::BIT_AMOUNT) - 1));
        res.map_err(|e| AppError::Unexpected(anyhow!(e)))
    }
}

impl FromRedisValue for PrivilegesNumber {
    fn from_redis_value(val: &redis::Value) -> redis::RedisResult<Self> {
        match val {
            redis::Value::Data(bytes) => {
                let returned_string = String::from_utf8(bytes.clone()).map_err(|_| RedisError::from((ErrorKind::TypeError, "expected valid UTF-8 string")))?;
                let parsed = returned_string.parse().map_err(|_| RedisError::from((ErrorKind::TypeError, "expected integer")))?;
                Ok(PrivilegesNumber::new(parsed))
            },
            _ => Err(RedisError::from((ErrorKind::TypeError, "expected \"string\" redis value"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(0b0000, CanSendMessages::No)]
    #[case(0b0001, CanSendMessages::No)]
    #[case(0b0010, CanSendMessages::Yes)]
    #[case(0b0100, CanSendMessages::SlowChat(SlowChat::OneSec))]
    #[case(0b0110, CanSendMessages::SlowChat(SlowChat::FiveSecs))]
    #[case(0b1000, CanSendMessages::SlowChat(SlowChat::ThirtySecs))]
    #[case(0b1010, CanSendMessages::SlowChat(SlowChat::OneMinute))]
    #[case(0b1100, CanSendMessages::SlowChat(SlowChat::FiveMinutes))]
    #[case(0b1110, CanSendMessages::SlowChat(SlowChat::FifteenMinutes))]
    fn get_can_send_messages(#[case] input: u8, #[case] exp: CanSendMessages) {
        let privilege_number = PrivilegesNumber::new(input);
        let res = privilege_number.get_privilege::<CanSendMessages>().unwrap();
        assert_eq!(res, exp)
    }

    #[rstest]
    #[case(0b0000, CanInvite::No)]
    #[case(0b0010, CanInvite::No)]
    #[case(0b0001, CanInvite::Yes)]
    fn get_can_invite(#[case] input: u8, #[case] exp: CanInvite) {
        let privilege_number = PrivilegesNumber::new(input);
        let res = privilege_number.get_privilege::<CanInvite>().unwrap();
        assert_eq!(res, exp)
    }

    #[rstest]
    #[case(0b1111, Privilege::CanInvite(CanInvite::No), 0b1110)]
    #[case(0b1110, Privilege::CanInvite(CanInvite::Yes), 0b1111)]
    #[case(0b1111, Privilege::CanInvite(CanInvite::Yes), 0b1111)]
    #[case(0b1010, Privilege::CanSendMessages(CanSendMessages::No), 0b0000)]
    #[case(0b1010, Privilege::CanSendMessages(CanSendMessages::Yes), 0b0010)]
    #[case(0b1011, Privilege::CanSendMessages(CanSendMessages::Yes), 0b0011)]
    fn set_privilege_number(#[case] init: u8, #[case] input: Privilege, #[case] exp: u8) {
        let privilege_number = PrivilegesNumber::new(init);
        let res = privilege_number.update_with(input);
        assert_eq!(res.inner(), exp)
    }
}
