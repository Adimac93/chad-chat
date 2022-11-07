use crate::auth::{get_token_secret, AuthError};
use anyhow::Context;
use axum::{
    async_trait,
    extract::{self, FromRequest},
};
use axum_extra::extract::CookieJar;
use jsonwebtoken::{decode, DecodingKey, Validation};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub id: Uuid,
    pub exp: u64,
}

#[async_trait]
impl<B> FromRequest<B> for Claims
where
    B: Send,
{
    type Rejection = AuthError;

    async fn from_request(req: &mut extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request(req).await.context("Failed to fetch cookie jar")?;
        let cookie = jar.get("jwt").ok_or(AuthError::InvalidToken)?;
        
        let mut validation = Validation::default();
        validation.leeway = 5;

        let data = decode::<Claims>(
            cookie.value(),
            &DecodingKey::from_secret(get_token_secret().expose_secret().as_bytes()),
           &validation
        );
        let new_data = data.map_err(|_| AuthError::InvalidToken)?;
        Ok(new_data.claims)
    }
}

#[derive(Serialize, Deserialize)]
pub struct AuthUser {
    pub login: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct NewGroup {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct GroupUser {
    pub user_id: Uuid,
    pub group_id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct NewMessage {
    pub content: String,
    pub user_id: Uuid,
    pub group_id: Uuid,
}
