use super::errors::*;
use anyhow::Context;
use argon2::hash_encoded;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use secrecy::{ExposeSecret, SecretString};

pub fn hash_pass(pass: SecretString) -> Result<String, AuthError> {
    Ok(hash_encoded(
        pass.expose_secret().as_bytes(),
        random_salt().as_bytes(),
        &argon2::Config::default(),
    )
    .context("Failed to hash pass")?)
}

pub fn random_salt() -> String {
    let mut rng = thread_rng();
    (0..8).map(|_| rng.sample(Alphanumeric) as char).collect()
}

pub fn pass_is_strong(user_password: &str, user_inputs: &[&str]) -> bool {
    let score = zxcvbn::zxcvbn(user_password, user_inputs);
    match score {
        Ok(s) => s.score() >= 3,
        Err(_) => false,
    }
}

pub fn get_token_secret() -> SecretString {
    SecretString::new(std::env::var("TOKEN_SECRET").expect("Cannot find token secret"))
}
