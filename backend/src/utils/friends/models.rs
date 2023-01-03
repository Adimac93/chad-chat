use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::utils::auth::ActivityStatus;

#[derive(Serialize, Deserialize, Debug)]
pub struct IdentifiedFriendIvitation {
    pub user_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FriendInvitationResponse {
    pub sender_id: Uuid,
    pub is_accepted: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Friend {
    pub note: String,
    pub status: ActivityStatus,
    pub profile_picture_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FriendList {
    pub friends: Vec<Friend>,
}
