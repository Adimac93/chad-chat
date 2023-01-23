use self::{errors::FriendError, models::FriendModel};
use super::auth::ActivityStatus;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Acquire, PgConnection, Postgres};
use uuid::Uuid;

pub mod errors;
pub mod models;

pub async fn send_friend_request_by_user_id<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
    request_user_id: Uuid,
) -> Result<(), FriendError> {
    let mut transaction = conn.begin().await?;

    let friend = Friend::new(user_id, request_user_id);
    if friend.is_friend(&mut transaction).await? {
        return Err(FriendError::AlreadyFriend);
    }

    let inv = Invitation::new(user_id, request_user_id);
    if inv.is_pending(&mut transaction).await? {
        return Err(FriendError::RequestSendAlready);
    }

    inv.send(&mut transaction).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn send_friend_request_by_username_and_tag<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
    tagged_username: TaggedUsername,
) -> Result<(), FriendError> {
    let mut transaction = conn.begin().await?;

    let Some(receiver_id) = tagged_username.id(&mut transaction).await? else {
        return Err(FriendError::UnknownUsername);
    };

    let friend = Friend::new(user_id, receiver_id);
    if friend.is_friend(&mut transaction).await? {
        return Err(FriendError::AlreadyFriend);
    }

    let inv = Invitation::new(user_id, receiver_id);
    if inv.is_pending(&mut transaction).await? {
        return Err(FriendError::RequestSendAlready);
    }

    inv.send(&mut transaction).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn fetch_friends<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
) -> Result<Vec<FriendModel>, FriendError> {
    let mut transaction = conn.begin().await?;
    let friends = User::new(user_id).friends(&mut transaction).await?;
    transaction.commit().await?;
    Ok(friends)
}

pub async fn respond_to_friend_request<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    sender_id: Uuid,
    receiver_id: Uuid,
    is_accepted: bool,
) -> Result<(), FriendError> {
    let mut transaction = conn.begin().await?;
    Invitation::new(sender_id, receiver_id)
        .respond(&mut transaction, is_accepted)
        .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn remove_friend<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
    friend_id: Uuid,
) -> Result<(), FriendError> {
    let mut transaction = conn.begin().await?;
    Friend::new(user_id, friend_id)
        .remove(&mut transaction)
        .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn update_friend_note<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
    friend_id: Uuid,
    note: String,
) -> Result<(), FriendError> {
    let mut transaction = conn.begin().await?;
    Friend::new(user_id, friend_id)
        .change_note(&mut transaction, note)
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
    pub async fn id(&self, conn: &mut PgConnection) -> Result<Option<Uuid>, FriendError> {
        let user_id = query!(
            r#"
                select id from users
                where username = $1 and tag = $2
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

pub struct Invitation {
    pub sender_id: Uuid,
    pub receiver_id: Uuid,
}

impl Invitation {
    pub fn new(sender_id: Uuid, receiver_id: Uuid) -> Self {
        Self {
            sender_id,
            receiver_id,
        }
    }

    pub async fn send(&self, conn: &mut PgConnection) -> Result<(), FriendError> {
        query!(
            r#"
                insert into friend_requests (sender_id, receiver_id)
                values ($1, $2)
            "#,
            self.sender_id,
            self.receiver_id
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    pub async fn respond(
        &self,
        conn: &mut PgConnection,
        is_accepted: bool,
    ) -> Result<(), FriendError> {
        query!(
            r#"
                delete from friend_requests
                where sender_id = $1 and receiver_id = $2
                returning *
            "#,
            self.sender_id,
            self.receiver_id,
        )
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(FriendError::RequestMissing)?;

        if !is_accepted {
            return Ok(());
        }

        Friend::new(self.sender_id, self.receiver_id)
            .add(&mut *conn)
            .await?;

        Ok(())
    }
    pub async fn is_pending(&self, conn: &mut PgConnection) -> Result<bool, FriendError> {
        let is_pending = query!(
            r#"
                select * from friend_requests
                where sender_id = $1 and receiver_id = $2
            "#,
            self.sender_id,
            self.receiver_id
        )
        .fetch_optional(&mut *conn)
        .await?
        .is_some();

        Ok(is_pending)
    }
}

pub struct Friend {
    pub user_id: Uuid,
    pub friend_id: Uuid,
}

impl Friend {
    fn new(user_id: Uuid, friend_id: Uuid) -> Self {
        Self { user_id, friend_id }
    }
    async fn add(&self, conn: &mut PgConnection) -> Result<(), FriendError> {
        let res = query!(
            r#"
                insert into user_friends (user_id, friend_id, note)
                values ($1, $2, '')
            "#,
            self.user_id,
            self.friend_id
        )
        .execute(&mut *conn)
        .await?;

        let res = query!(
            r#"
                insert into user_friends (user_id, friend_id, note)
                values ($1, $2, '')
            "#,
            self.friend_id,
            self.user_id,
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    async fn remove(&self, conn: &mut PgConnection) -> Result<(), FriendError> {
        query!(
            r#"
                delete from user_friends
                where 
                (user_id = $1 and friend_id = $2)
                or 
                (user_id = $2 and friend_id = $1)
            "#,
            self.user_id,
            self.friend_id
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    pub async fn change_note(
        &self,
        conn: &mut PgConnection,
        note: String,
    ) -> Result<(), FriendError> {
        query!(
            r#"
                update user_friends
                set note = $1
                where user_id = $2 and friend_id = $3
            "#,
            note,
            self.user_id,
            self.friend_id
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }
    pub async fn is_friend(&self, conn: &mut PgConnection) -> Result<bool, FriendError> {
        let is_friend = query!(
            r#"
                select * from user_friends
                where user_id = $1 and friend_id = $2
            "#,
            self.user_id,
            self.friend_id
        )
        .fetch_optional(&mut *conn)
        .await?
        .is_some();

        Ok(is_friend)
    }
}

pub struct User {
    user_id: Uuid,
}

impl User {
    pub fn new(user_id: Uuid) -> Self {
        Self { user_id }
    }

    pub async fn friends(&self, conn: &mut PgConnection) -> Result<Vec<FriendModel>, FriendError> {
        let friends = query_as!(
        FriendModel,
        r#"
            select users.activity_status as "status: ActivityStatus", users.profile_picture_url, user_friends.note from user_friends
            join users on users.id = user_friends.friend_id
            where user_id = $1
        "#,
        self.user_id
        )
        .fetch_all(&mut *conn)
        .await?;

        Ok(friends)
    }
}
