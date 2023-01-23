use crate::utils::auth::errors::AuthError;
use axum::{
    async_trait,
    extract::{self, FromRequest},
};
use secrecy::Secret;

#[derive(Clone)]
pub struct JwtAccessSecret(pub Secret<String>);

#[derive(Clone)]
pub struct JwtRefreshSecret(pub Secret<String>);

#[derive(Clone)]
pub struct TokenExtractors {
    pub access: JwtAccessSecret,
    pub refresh: JwtRefreshSecret,
}

#[async_trait]
impl<B> FromRequest<B> for TokenExtractors
where
    B: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request(req: &mut extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        Ok(req
            .extensions()
            .get::<Self>()
            .expect("Failed to get jwt secret extension")
            .clone())
    }
}
