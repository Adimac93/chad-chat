use axum::{
    async_trait,
    extract::{ConnectInfo, FromRequest, RequestParts},
    response::IntoResponse,
};
use hyper::StatusCode;
use sqlx::types::ipnetwork::IpNetwork;
use tracing::error;

use crate::modules::external_api::{GeolocationData, HttpClient};

use super::addr::ClientAddr;

#[derive(Debug)]
pub struct NetworkData {
    pub net: IpNetwork,
    pub geo: GeolocationData,
}

#[async_trait]
impl<B> FromRequest<B> for NetworkData
where
    B: Send + std::marker::Sync,
{
    type Rejection = hyper::StatusCode;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let net = req
            .extract::<ConnectInfo<ClientAddr>>()
            .await
            .map_err(|e| {
                error!("Faield to get client ip");
                e.into_response().status()
            })?
            .0
            .network();

        if let Some(http_client) = req.extensions().get::<HttpClient>() {
            let geo = http_client.fetch_geolocation(net.ip()).await.map_err(|e| {
                error!("Faield to fetch geolocation: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            return Ok(Self { net, geo });
        } else {
            error!("Failed to get http client");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}
