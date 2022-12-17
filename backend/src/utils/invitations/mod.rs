pub mod errors;
pub mod models;

use anyhow::Context;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Acquire, PgPool, Postgres};
use time::{Duration, OffsetDateTime};
use tracing::debug;
use uuid::Uuid;

use self::errors::InvitationError;

use super::groups::{models::GroupInfo, try_add_user_to_group};

// Frontend payload
#[derive(Deserialize, Debug)]
pub struct GroupInvitationCreate {
    group_id: Uuid,
    expiration_index: Option<i32>,
    usage_index: Option<i32>,
}

impl TryFrom<GroupInvitationCreate> for GroupInvitation {
    type Error = InvitationError;
    fn try_from(value: GroupInvitationCreate) -> Result<Self, Self::Error> {
        let exp = match value.expiration_index {
            Some(i) => Some(Expiration::try_from(i)?),
            None => None,
        };
        let uses = match value.usage_index {
            Some(i) => Some(Uses::try_from(i)?),
            None => None,
        };

        let invitation = GroupInvitation::new(value.group_id, exp, uses);
        Ok(invitation)
    }
}

#[derive(Serialize)]
pub struct GroupInvitation {
    group_id: Uuid,
    expiration_date: Option<OffsetDateTime>,
    uses_left: Option<i32>,
    id: String,
}

impl GroupInvitation {
    fn new(group_id: Uuid, expiration_time: Option<Expiration>, uses_left: Option<Uses>) -> Self {
        GroupInvitation {
            group_id,
            expiration_date: expiration_time
                .and_then(|time| Some(OffsetDateTime::now_utc() + Duration::from(time))),
            uses_left: uses_left.and_then(|uses| Some(i32::from(uses))),
            id: nanoid!(10),
        }
    }
}

pub async fn try_create_group_invitation_with_code(
    pool: &PgPool,
    user_id: &Uuid,
    invitation: GroupInvitationCreate,
) -> Result<String, InvitationError> {
    debug!("{invitation:#?}");
    let invitation = GroupInvitation::try_from(invitation)?;
    query!(
        r#"
            insert into group_invitations
            (
            user_id, group_id,
            id, expiration_date, uses_left
            )
            values ($1, $2, $3, $4, $5)
        "#,
        &user_id,
        invitation.group_id,
        invitation.id,
        invitation.expiration_date,
        invitation.uses_left
    )
    .execute(pool)
    .await
    .context("Failed to create a group invitation")?;

    Ok(invitation.id)
}

pub async fn fetch_group_info_by_code(
    pool: &PgPool,
    code: &str,
) -> Result<GroupInfo, InvitationError> {
    let mut transaction = pool.begin().await.context("Failed to begin transaction")?;
    let res = query!(
        r#"
            select groups.name, groups.id as group_id, count(*) as members_count from group_invitations
            join groups on groups.id = group_invitations.group_id
            join group_users on groups.id = group_users.group_id
            where group_invitations.id = $1
            group by groups.id
        "#,
        code,
    )
    .fetch_optional(&mut transaction)
    .await
    .context("Failed to find group invitation")?;

    let invitation = res.ok_or(InvitationError::InvalidCode)?;

    Ok(GroupInfo {
        name: invitation.name,
        members: invitation.members_count.context("Members count is None")?,
    })
}

pub async fn try_join_group_by_code<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: &Uuid,
    code: &str,
) -> Result<(), InvitationError> {
    let mut transaction = conn.begin().await.context("Failed to begin transaction")?;
    let Some(invitation) = query_as!(
        GroupInvitation,
        r#"
            select group_id, expiration_date, id, uses_left from group_invitations
            where id = $1
        "#,
        code
    )
    .fetch_optional(&mut transaction)
    .await
    .context("Failed to find group invitation")? else {
        return Err(InvitationError::InvalidCode)
    };

    match invitation.uses_left {
        Some(use_number) if use_number <= 0 => {
            let _res = query!(
                r"
                    delete from group_invitations
                    where id = $1
                ",
                invitation.id
            )
            .execute(&mut transaction)
            .await
            .context("Failed to delete expired group invitation")?;

            transaction
                .commit()
                .await
                .context("Failed to commit transaction")?;

            return Err(InvitationError::InvitationExpired);
        }
        _ => (),
    }

    match invitation.expiration_date {
        Some(expiry) if expiry < OffsetDateTime::now_utc() => {
            let _res = query!(
                r"
                    delete from group_invitations
                    where id = $1
                ",
                invitation.id
            )
            .execute(&mut transaction)
            .await
            .context("Failed to delete expired group invitation")?;

            transaction
                .commit()
                .await
                .context("Failed to commit transaction")?;

            return Err(InvitationError::InvitationExpired);
        }
        _ => (),
    }

    try_add_user_to_group(&mut transaction, user_id, &invitation.group_id)
        .await
        .context("Failed to add user to group")?; // ? better error conversion possible

    if let Some(use_number) = invitation.uses_left {
        let _res = query!(
            r"
                update group_invitations
                set uses_left = $1
                where id = $2
            ",
            use_number - 1,
            invitation.id,
        )
        .execute(&mut transaction)
        .await
        .context("Failed to update group invitation uses_left field")?;
    }

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;
    return Ok(());
}

#[derive(Debug)]
enum Uses {
    One,
    Five,
    Ten,
    TwentyFive,
    Fifty,
    OneHundred,
}

impl From<Uses> for i32 {
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

impl TryFrom<i32> for Uses {
    type Error = InvitationError;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Uses::One),
            1 => Ok(Uses::Five),
            2 => Ok(Uses::Ten),
            3 => Ok(Uses::TwentyFive),
            4 => Ok(Uses::Fifty),
            5 => Ok(Uses::OneHundred),
            _n => Err(InvitationError::UnsupportedVariant),
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

impl TryFrom<i32> for Expiration {
    type Error = InvitationError;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Expiration::HalfHour),
            1 => Ok(Expiration::Hour),
            2 => Ok(Expiration::QuarterDay),
            3 => Ok(Expiration::HalfDay),
            4 => Ok(Expiration::Day),
            5 => Ok(Expiration::Week),
            _n => Err(InvitationError::UnsupportedVariant),
        }
    }
}
