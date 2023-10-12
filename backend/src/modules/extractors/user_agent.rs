use axum::{
    async_trait,
    extract::{FromRequest, FromRequestParts},
    http::{self, request::Parts},
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{modules::external_api::{HttpClient, UserAgentParsed}, AppState};

#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "user_agent_data")]
pub struct UserAgentData {
    browser: String,
    device_brand_name: String,
    device_name: String,
    device_type: String,
    platform: String,
    crawler: bool,
    is_fake: bool,
}

impl UserAgentData {
    pub fn new(ua: UserAgentParsed) -> Self {
        Self {
            browser: ua.browser,
            device_brand_name: ua.device_brand_name,
            device_name: ua.device_name,
            device_type: ua.device_type,
            platform: ua.platform,
            crawler: ua.crawler,
            is_fake: ua.isfake,
        }
    }
    pub fn is_trusted(&self) -> bool {
        !(self.crawler || self.is_fake)
    }
}

#[async_trait]
impl FromRequestParts<AppState> for UserAgentData {
    type Rejection = hyper::StatusCode;

    async fn from_request_parts(req: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user_agent_header = req
            .headers
            .get(http::header::USER_AGENT)
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        let user_agent = state.client
            .parse_user_agent(user_agent_header.to_str().unwrap())
            .await
            .unwrap();

        Ok(UserAgentData::new(user_agent))
    }
}
