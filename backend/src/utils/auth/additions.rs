use anyhow::anyhow;
use argon2::password_hash::SaltString;
use argon2::{password_hash, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::seq::IteratorRandom;
use rand::thread_rng;
use std::collections::HashSet;

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(rand::thread_rng());
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!(e).context("failed to hash password"))?
        .to_string())
}

pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    let hash = PasswordHash::new(hash).map_err(|e| anyhow!(e).context("password hash invalid"))?;
    let res = Argon2::default().verify_password(password.as_bytes(), &hash);
    match res {
        Ok(()) => Ok(true),
        Err(password_hash::Error::Password) => Ok(false),
        Err(e) => Err(anyhow!(e)),
    }
}

pub fn is_strong_password(user_password: &str, user_inputs: &[&str]) -> bool {
    let score = zxcvbn::zxcvbn(user_password, user_inputs);
    score.map_or(false, |entropy| entropy.score() >= 3)
}

pub fn random_username_tag(used_tags: HashSet<i32>) -> Option<i32> {
    let mut rng = thread_rng();
    (0..10000)
        .filter(|x| !used_tags.contains(x))
        .choose(&mut rng)
}
