use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Acquire, Postgres};
use uuid::Uuid;

use self::{errors::FriendError, models::Friend};

use super::{auth::ActivityStatus, groups::check_if_user_exists};

pub mod errors;
pub mod models;

pub async fn send_friend_request<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
    request_user_id: Uuid,
) -> Result<(), FriendError> {
    let mut transaction = conn.begin().await.context("Failed to begin transaction")?;

    //check_if_user_exists(transaction, &request_user_id);
    //? is a friend already
    let res = query!(
        r#"
            select * from user_friends
            where user_id = $1 and friend_id = $2
        "#,
        user_id,
        request_user_id
    )
    .fetch_optional(&mut transaction)
    .await
    .context("Failed to select user friend")?;

    if let Some(_) = res {
        return Err(FriendError::AlreadyFriend);
    }

    //? is invitation pending
    let res = query!(
        r#"
            select * from friend_requests
            where sender_id = $1 and receiver_id = $2
        "#,
        user_id,
        request_user_id
    )
    .fetch_optional(&mut transaction)
    .await
    .context("Failed to select friend request")?;

    if let Some(_) = res {
        return Err(FriendError::RequestSendAlready);
    }

    query!(
        r#"
            insert into friend_requests (sender_id, receiver_id)
            values ($1, $2)
        "#,
        user_id,
        request_user_id
    )
    .execute(&mut transaction)
    .await
    .context("Failed to create a friend request")?;

    transaction.commit().await.context("Transaction failed")?;

    Ok(())
}

pub async fn fetch_user_friends<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
) -> Result<Vec<Friend>, FriendError> {
    let mut transaction = conn.begin().await.context("Failed to begin transaction")?;

    let friends = query_as!(
        Friend,
        r#"
            select users.activity_status as "status: ActivityStatus", user_friends.note from user_friends
            join users on users.id = user_friends.friend_id
            where user_id = $1
        "#,
        user_id
    )
    .fetch_all(&mut transaction)
    .await
    .context("Failed to fetch friends")?;

    transaction.commit().await.context("Transaction failed")?;

    Ok(friends)
}

pub async fn respond_to_friend_request<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    is_accepted: bool,
    sender_id: Uuid,
    receiver_id: Uuid,
) -> Result<(), FriendError> {
    let mut transaction = conn.begin().await.context("Failed to begin transaction")?;

    //? is request present
    let res = query!(
        r#"
            select * from friend_requests
            where sender_id = $1 and receiver_id = $2
        "#,
        sender_id,
        receiver_id
    )
    .fetch_optional(&mut transaction)
    .await
    .context("Failed to fetch friend requests")?;

    if let None = res {
        return Err(FriendError::AlreadyFriend);
    }

    // delete request
    query!(
        r#"
            delete from friend_requests
            where sender_id = $1 and receiver_id = $2
        "#,
        sender_id,
        receiver_id
    )
    .execute(&mut transaction)
    .await
    .context("Failed to delete friend request")?;

    // commit and return if declined
    if !is_accepted {
        transaction.commit().await.context("Transaction failed")?;
        return Ok(());
    }

    // add friends
    let res = query!(
        r#"
            insert into user_friends (user_id, friend_id, note)
            values ($1, $2, '')
        "#,
        sender_id,
        receiver_id
    )
    .execute(&mut transaction)
    .await;

    if let Err(e) = res {
        transaction
            .rollback()
            .await
            .context("Failed to abort transaction")?;
        return Err(FriendError::Unexpected(e.into()));
    }

    let res = query!(
        r#"
            insert into user_friends (user_id, friend_id, note)
            values ($1, $2, '')
        "#,
        receiver_id,
        sender_id,
    )
    .execute(&mut transaction)
    .await;

    if let Err(e) = res {
        transaction
            .rollback()
            .await
            .context("Failed to abort transaction")?;
        return Err(FriendError::Unexpected(e.into()));
    }

    transaction.commit().await.context("Transaction failed")?;

    return Ok(());
}

pub async fn remove_friend<'c>(
    conn: impl Acquire<'c, Database = Postgres>,
    user_id: Uuid,
    friend_id: Uuid,
) -> Result<(), FriendError> {
    let mut transaction = conn
        .begin()
        .await
        .context("Failed to abort the transaction")?;

    let res = query!(
        r#"
            delete from user_friends
            where 
            (user_id = $1 and friend_id = $2)
            or 
            (user_id = $2 and friend_id = $1)
        "#,
        user_id,
        friend_id
    )
    .execute(&mut transaction)
    .await;

    if let Err(e) = res {
        transaction.rollback().await.context("Rollback failed")?;
        return Err(FriendError::Unexpected(e.into()));
    }

    transaction.commit().await.context("Transaction failed")?;

    Ok(())
}