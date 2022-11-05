use axum::{async_trait, extract::{FromRequest, self}, TypedHeader, headers::{Authorization, authorization::Bearer}};
use jsonwebtoken::{decode, Validation, DecodingKey};
use secrecy::ExposeSecret;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::auth::{get_token_secret, AuthError};

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub id: Uuid,
    pub exp: u64,
}

#[async_trait]
impl<B> FromRequest<B> for Claims
where B: Send,
{
    type Rejection = AuthError;

    async fn from_request(req: &mut extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = 
            TypedHeader::<Authorization<Bearer>>::from_request(req).await.map_err(|_| AuthError::InvalidToken)?;
        let data = decode::<Claims>(bearer.token(), &DecodingKey::from_secret(get_token_secret().expose_secret().as_bytes()), &Validation::default());
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