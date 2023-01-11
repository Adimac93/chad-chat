use argon2::hash_encoded;
use rand::seq::{IteratorRandom, SliceRandom};
use rand::{
    distributions::{Alphanumeric, Uniform},
    thread_rng, Rng,
};
use secrecy::{ExposeSecret, SecretString};

pub fn hash_pass(pass: SecretString) -> Result<String, argon2::Error> {
    hash_encoded(
        pass.expose_secret().as_bytes(),
        random_salt().as_bytes(),
        &argon2::Config::default(),
    )
}

pub fn random_salt() -> String {
    let mut rng = thread_rng();
    (0..8).map(|_| rng.sample(Alphanumeric) as char).collect()
}

pub fn pass_is_strong(user_password: &str, user_inputs: &[&str]) -> bool {
    let score = zxcvbn::zxcvbn(user_password, user_inputs);
    score.map_or(false, |entropy| entropy.score() >= 3)
}

pub fn random_username_tag(used_tags: Vec<i32>) -> Option<i32> {
    let mut rng = thread_rng();
    (0..10000)
        .filter(|x| !used_tags.contains(x))
        .choose(&mut rng)
}
