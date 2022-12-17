use serde::Serialize;

use crate::utils::auth::ActivityStatus;

#[derive(Serialize, Debug)]
pub struct FriendRequest {
    pub login: String,
}

#[derive(Serialize, Debug)]
pub struct FriendRequestResponse {
    pub is_accepted: bool,
}

#[derive(Serialize, Debug)]
pub struct Friend {
    pub note: String,
    pub status: ActivityStatus,
}
