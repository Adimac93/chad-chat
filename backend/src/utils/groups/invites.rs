use nanoid::nanoid;
use std::collections::HashMap;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

struct Invitations<T>(HashMap<String, T>);
impl Invitations<GroupInvitation> {
    fn new() -> Self {
        Invitations(HashMap::new())
    }
}
impl Invitations<GroupInvitation> {
    fn add(&mut self, group_invitation: GroupInvitation) {
        let Invitations(invitations) = self;
        let id = nanoid!(10);
        invitations.insert(id, group_invitation);
    }
    fn check(&mut self, invitation_id: String, user_id: &Uuid) -> Option<bool> {
        let Invitations(invitations) = self;
        if let Some(group_invitation) = invitations.get(&invitation_id) {
            if let Some(exp) = &group_invitation.expiration_date {
                if exp < &OffsetDateTime::now_utc() {
                    invitations.remove(&invitation_id);
                    return Some(false);
                }
            }
            if let Some(ussage) = &group_invitation.ussage {
                if ussage.current < ussage.max {
                    invitations.remove(&invitation_id);
                    return Some(false);
                }
            }
            return Some(true);
        }
        None
    }
}

struct GroupInvitation {
    group_id: Uuid,
    expiration_date: Option<OffsetDateTime>,
    ussage: Option<Ussage>,
}

struct Ussage {
    max: u8,
    current: u8,
}

impl Ussage {
    fn new(max: Uses) -> Self {
        Ussage {
            max: u8::from(max),
            current: 0,
        }
    }
}

enum Uses {
    One,
    Five,
    Ten,
    TwentyFive,
    Fifty,
    OneHundred,
}

impl From<Uses> for u8 {
    fn from(uses: Uses) -> Self {
        match uses {
            Uses::One => 1,
            Uses::Five => 5,
            Uses::Ten => 10,
            Uses::TwentyFive => 25,
            Uses::Fifty => 50,
            Uses::OneHundred => 100,
        }
    }
}

impl TryFrom<u8> for Uses {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Uses::One),
            1 => Ok(Uses::Five),
            2 => Ok(Uses::Ten),
            3 => Ok(Uses::TwentyFive),
            4 => Ok(Uses::Fifty),
            5 => Ok(Uses::OneHundred),
            n => Err(format!("Index: {n} usupported")),
        }
    }
}

enum Expiration {
    HalfHour,
    Hour,
    QuarterDay,
    HalfDay,
    Day,
    Week,
}

impl From<Expiration> for Duration {
    fn from(exp: Expiration) -> Self {
        match exp {
            Expiration::HalfHour => Duration::minutes(30),
            Expiration::Hour => Duration::hours(1),
            Expiration::QuarterDay => Duration::hours(6),
            Expiration::HalfDay => Duration::hours(12),
            Expiration::Day => Duration::days(1),
            Expiration::Week => Duration::weeks(1),
        }
    }
}

impl TryFrom<u8> for Expiration {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Expiration::HalfHour),
            1 => Ok(Expiration::Hour),
            2 => Ok(Expiration::QuarterDay),
            3 => Ok(Expiration::HalfDay),
            4 => Ok(Expiration::Day),
            5 => Ok(Expiration::Week),
            n => Err(format!("Index: {n} usupported")),
        }
    }
}

impl GroupInvitation {
    fn new(
        group_id: Uuid,
        expiration_time: Option<Expiration>,
        max_number_of_uses: Option<Uses>,
    ) -> Self {
        GroupInvitation {
            group_id,
            expiration_date: expiration_time
                .and_then(|time| Some(OffsetDateTime::now_utc() + Duration::from(time))),
            ussage: max_number_of_uses.and_then(|uses| Some(Ussage::new(uses))),
        }
    }
}
