use crate::{utils::auth::errors::AuthError, AppState};
use axum::{
    async_trait,
    extract::{self, FromRequest, FromRequestParts}, http::request::Parts,
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
