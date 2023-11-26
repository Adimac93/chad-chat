use self::models::FriendModel;
use super::auth::ActivityStatus;
use crate::errors::AppError;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Acquire, PgConnection, Postgres};
use uuid::Uuid;

pub mod models;

pub async fn send_friend_request_by_user_id<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
    request_user_id: Uuid,
) -> Result<(), AppError> {
    let mut transaction = conn.begin().await?;

    let mut friend = Friend::new(user_id, request_user_id, &mut transaction);
    if friend.is_friend().await? {
        return Err(AppError::exp(StatusCode::BAD_REQUEST, "Already a friend"));
    }

    let mut inv = Invitation::new(user_id, request_user_id, &mut transaction);
    if inv.is_pending().await? {
        return Err(AppError::exp(
            StatusCode::BAD_REQUEST,
            "Friend request already sent",
        ));
    }

    inv.send().await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn send_friend_request_by_username_and_tag<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
    tagged_username: TaggedUsername,
) -> Result<(), AppError> {
    let mut transaction = conn.begin().await?;

    let Some(receiver_id) = tagged_username.id(&mut transaction).await? else {
        return Err(AppError::exp(StatusCode::BAD_REQUEST, "Unknown username"));
    };

    let mut friend = Friend::new(user_id, receiver_id, &mut transaction);
    if friend.is_friend().await? {
        return Err(AppError::exp(StatusCode::BAD_REQUEST, "Already a friend"));
    }

    let mut inv = Invitation::new(user_id, receiver_id, &mut transaction);
    if inv.is_pending().await? {
        return Err(AppError::exp(
            StatusCode::BAD_REQUEST,
            "Friend request already sent",
        ));
    }

    inv.send().await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn fetch_friends<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
) -> Result<Vec<FriendModel>, AppError> {
    let mut transaction = conn.begin().await?;
    let friends = User::new(user_id, &mut transaction).friends().await?;
    transaction.commit().await?;
    Ok(friends)
}

pub async fn respond_to_friend_request<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    sender_id: Uuid,
    receiver_id: Uuid,
    is_accepted: bool,
) -> Result<(), AppError> {
    let mut transaction = conn.begin().await?;
    Invitation::new(sender_id, receiver_id, &mut transaction)
        .respond(is_accepted)
        .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn remove_friend<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
    friend_id: Uuid,
) -> Result<(), AppError> {
    let mut transaction = conn.begin().await?;
    Friend::new(user_id, friend_id, &mut transaction)
        .remove()
        .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn update_friend_note<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
    friend_id: Uuid,
    note: String,
) -> Result<(), AppError> {
    let mut transaction = conn.begin().await?;
    Friend::new(user_id, friend_id, &mut transaction)
        .change_note(note)
        .await?;
    transaction.commit().await?;
    Ok(())
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaggedUsername {
    pub username: String,
    pub tag: u16,
}

impl TaggedUsername {
    pub async fn id(&self, conn: &mut PgConnection) -> sqlx::Result<Option<Uuid>> {
        let user_id = query!(
            r#"
                SELECT id FROM users
                WHERE username = $1 AND tag = $2
            "#,
            self.username,
            self.tag as i32
        )
        .fetch_optional(&mut *conn)
        .await?
        .map(|res| res.id);

        Ok(user_id)
    }
}

pub struct Invitation<'c> {
    pub sender_id: Uuid,
    pub receiver_id: Uuid,
    conn: &'c mut PgConnection,
}

impl<'c> Invitation<'c> {
    pub fn new(sender_id: Uuid, receiver_id: Uuid, conn: &'c mut PgConnection) -> Self {
        Self {
            sender_id,
            receiver_id,
            conn,
        }
    }

    pub async fn send(&mut self) -> sqlx::Result<()> {
        query!(
            r#"
                INSERT INTO friend_requests (sender_id, receiver_id)
                VALUES ($1, $2)
            "#,
            self.sender_id,
            self.receiver_id
        )
        .execute(&mut *self.conn)
        .await?;

        Ok(())
    }

    pub async fn respond(&mut self, is_accepted: bool) -> Result<(), AppError> {
        query!(
            r#"
                DELETE FROM friend_requests
                WHERE sender_id = $1 AND receiver_id = $2
                RETURNING *
            "#,
            self.sender_id,
            self.receiver_id,
        )
        .fetch_optional(&mut *self.conn)
        .await?
        .ok_or(AppError::exp(
            StatusCode::BAD_REQUEST,
            "Friend request is missing",
        ))?;

        if !is_accepted {
            return Ok(());
        }

        Friend::new(self.sender_id, self.receiver_id, &mut *self.conn)
            .add()
            .await?;

        Ok(())
    }

    pub async fn is_pending(&mut self) -> sqlx::Result<bool> {
        let is_pending = query!(
            r#"
                SELECT * FROM friend_requests
                WHERE sender_id = $1 AND receiver_id = $2
            "#,
            self.sender_id,
            self.receiver_id
        )
        .fetch_optional(&mut *self.conn)
        .await?
        .is_some();

        Ok(is_pending)
    }
}

pub struct Friend<'c> {
    pub user_id: Uuid,
    pub friend_id: Uuid,
    conn: &'c mut PgConnection,
}

impl<'c> Friend<'c> {
    fn new(user_id: Uuid, friend_id: Uuid, conn: &'c mut PgConnection) -> Self {
        Self {
            user_id,
            friend_id,
            conn,
        }
    }

    async fn add(&mut self) -> sqlx::Result<()> {
        let mut tr = self.conn.begin().await?;

        let _res = query!(
            r#"
                INSERT INTO user_friends (user_id, friend_id, note)
                VALUES ($1, $2, '')
            "#,
            self.user_id,
            self.friend_id
        )
        .execute(&mut *tr)
        .await?;

        let _res = query!(
            r#"
                INSERT INTO user_friends (user_id, friend_id, note)
                VALUES ($1, $2, '')
            "#,
            self.friend_id,
            self.user_id,
        )
        .execute(&mut *tr)
        .await?;

        tr.commit().await?;

        Ok(())
    }

    async fn remove(&mut self) -> sqlx::Result<()> {
        query!(
            r#"
                DELETE FROM user_friends
                WHERE 
                (user_id = $1 AND friend_id = $2)
                or 
                (user_id = $2 AND friend_id = $1)
            "#,
            self.user_id,
            self.friend_id
        )
        .execute(&mut *self.conn)
        .await?;

        Ok(())
    }

    pub async fn change_note(&mut self, note: String) -> sqlx::Result<()> {
        query!(
            r#"
                UPDATE user_friends
                SET note = $1
                WHERE user_id = $2 AND friend_id = $3
            "#,
            note,
            self.user_id,
            self.friend_id
        )
        .execute(&mut *self.conn)
        .await?;

        Ok(())
    }

    pub async fn is_friend(&mut self) -> sqlx::Result<bool> {
        let is_friend = query!(
            r#"
                SELECT * FROM user_friends
                WHERE user_id = $1 AND friend_id = $2
            "#,
            self.user_id,
            self.friend_id
        )
        .fetch_optional(&mut *self.conn)
        .await?
        .is_some();

        Ok(is_friend)
    }
}

pub struct User<'c> {
    user_id: Uuid,
    conn: &'c mut PgConnection,
}

impl<'c> User<'c> {
    pub fn new(user_id: Uuid, conn: &'c mut PgConnection) -> Self {
        Self { user_id, conn }
    }

    pub async fn friends(&mut self) -> sqlx::Result<Vec<FriendModel>> {
        todo!()
    }
}
