pub mod models;

use crate::errors::AppError;
use anyhow::Context;
use hyper::StatusCode;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Acquire, PgPool, Postgres};
use time::{Duration, OffsetDateTime};
use tracing::debug;
use uuid::Uuid;

use super::groups::{models::GroupInfo, try_add_user_to_group};

// Frontend payload
#[derive(Deserialize, Debug)]
pub struct GroupInvitationCreate {
    group_id: Uuid,
    expiration_index: Option<i32>,
    usage_index: Option<i32>,
}

impl TryFrom<GroupInvitationCreate> for GroupInvitation {
    type Error = AppError;
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
) -> Result<String, AppError> {
    debug!("{invitation:#?}");
    let invitation = GroupInvitation::try_from(invitation)?;
    query!(
        r#"
            INSERT INTO group_invitations
            (
            user_id, group_id,
            id, expiration_date, uses_left
            )
            VALUES ($1, $2, $3, $4, $5)
        "#,
        &user_id,
        invitation.group_id,
        invitation.id,
        invitation.expiration_date,
        invitation.uses_left
    )
    .execute(pool)
    .await?;

    Ok(invitation.id)
}

pub async fn fetch_group_info_by_code(pool: &PgPool, code: &str) -> Result<GroupInfo, AppError> {
    let mut transaction = pool.begin().await?;
    let res = query!(
        r#"
            SELECT groups.name, groups.id as group_id, count(*) as members_count FROM group_invitations
            JOIN groups ON groups.id = group_invitations.group_id
            JOIN group_users ON groups.id = group_users.group_id
            WHERE group_invitations.id = $1
            GROUP BY groups.id
        "#,
        code,
    )
    .fetch_optional(&mut *transaction)
    .await?;

    let invitation = res.ok_or(AppError::exp(
        StatusCode::BAD_REQUEST,
        "Invalid group invitation code",
    ))?;

    Ok(GroupInfo {
        name: invitation.name,
        members: invitation
            .members_count
            .context("Members count is None")
            .map_err(AppError::Unexpected)?, // to change
    })
}

pub async fn try_join_group_by_code<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: &Uuid,
    code: &str,
) -> Result<(), AppError> {
    let mut transaction = conn.begin().await?;

    let Some(invitation) = query_as!(
        GroupInvitation,
        r#"
            SELECT group_id, expiration_date, id, uses_left from group_invitations
            WHERE id = $1
        "#,
        code
    )
    .fetch_optional(&mut *transaction)
    .await?
    else {
        return Err(AppError::exp(
            StatusCode::BAD_REQUEST,
            "Invalid group invitation code",
        ))?;
    };

    match invitation.uses_left {
        Some(use_number) if use_number <= 0 => {
            let _res = query!(
                r"
                    DELETE FROM group_invitations
                    WHERE id = $1
                ",
                invitation.id
            )
            .execute(&mut *transaction)
            .await?;

            transaction.commit().await?;

            return Err(AppError::exp(
                StatusCode::BAD_REQUEST,
                "Invitation is expired",
            ))?;
        }
        _ => (),
    }

    match invitation.expiration_date {
        Some(expiry) if expiry < OffsetDateTime::now_utc() => {
            let _res = query!(
                r"
                    DELETE FROM group_invitations
                    WHERE id = $1
                ",
                invitation.id
            )
            .execute(&mut *transaction)
            .await?;

            transaction.commit().await?;

            return Err(AppError::exp(
                StatusCode::BAD_REQUEST,
                "Invitation is expired",
            ))?;
        }
        _ => (),
    }

    try_add_user_to_group(&mut transaction, user_id, &invitation.group_id).await?; // ? better error conversion possible

    if let Some(use_number) = invitation.uses_left {
        let _res = query!(
            r"
                UPDATE group_invitations
                SET uses_left = $1
                WHERE id = $2
            ",
            use_number - 1,
            invitation.id,
        )
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await?;
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
    type Error = AppError;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Uses::One),
            1 => Ok(Uses::Five),
            2 => Ok(Uses::Ten),
            3 => Ok(Uses::TwentyFive),
            4 => Ok(Uses::Fifty),
            5 => Ok(Uses::OneHundred),
            _n => Err(AppError::exp(
                StatusCode::BAD_REQUEST,
                "Unsupported invitation variant",
            )),
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
    type Error = AppError;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Expiration::HalfHour),
            1 => Ok(Expiration::Hour),
            2 => Ok(Expiration::QuarterDay),
            3 => Ok(Expiration::HalfDay),
            4 => Ok(Expiration::Day),
            5 => Ok(Expiration::Week),
            _n => Err(AppError::exp(
                StatusCode::BAD_REQUEST,
                "Unsupported invitation variant",
            )),
        }
    }
}
