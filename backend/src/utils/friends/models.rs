use serde::{Deserialize, Serialize};
use typeshare::typeshare;
use uuid::Uuid;

use crate::utils::auth::ActivityStatus;

#[typeshare]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentifiedFriendIvitation {
    pub user_id: Uuid,
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug)]
pub struct FriendInvitationResponse {
    pub sender_id: Uuid,
    pub is_accepted: bool,
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug)]
pub struct FriendModel {
    pub note: String,
    pub status: ActivityStatus,
    pub profile_picture_url: String,
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug)]
pub struct FriendList {
    pub friends: Vec<FriendModel>,
}
