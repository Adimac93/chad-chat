use axum::{
    async_trait,
    extract::{ConnectInfo, FromRequestParts},
    http::request::Parts,
    response::IntoResponse,
    RequestPartsExt,
};
use hyper::StatusCode;

use sqlx::types::ipnetwork::IpNetwork;
use tracing::error;

use crate::{
    modules::external_api::{GeolocationData},
    state::AppState,
};

use super::addr::ClientAddr;

#[derive(Debug)]
pub struct NetworkData {
    pub ip: IpNetwork,
    pub geolocation_data: GeolocationData,
}

#[async_trait]
impl FromRequestParts<AppState> for NetworkData {
    type Rejection = hyper::StatusCode;

    async fn from_request_parts(
        req: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let net = req
            .extract::<ConnectInfo<ClientAddr>>()
            .await
            .map_err(|e| {
                error!("Faield to get client ip");
                e.into_response().status()
            })?
            .0
            .network();

        let geo = state
            .client
            .fetch_geolocation(net.ip())
            .await
            .map_err(|e| {
                error!("Faield to fetch geolocation: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        return Ok(Self {
            ip: net,
            geolocation_data: geo,
        });
    }
}
