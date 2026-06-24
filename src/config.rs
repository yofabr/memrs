use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use color_eyre::Result;
use rand::rngs::OsRng;
use std::env;

type Int = u16;

#[derive(Debug, Clone, Default)]
pub struct Config {
    port: Int,
    password: String,
    // size:
}

impl Config {
    pub fn load_config() -> Self {
        let mut config = Self {
            port: 7898,
            password: String::new(),
        };

        if let Ok(port) = env::var("PORT") {
            config.port = port.parse::<u16>().unwrap();
        }
        if let Ok(password) = env::var("PASSWORD") {
            config.password = hash_password(&password).unwrap_or_default();
        }
        config
    }
}

pub fn hash_password(pass: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2.hash_password(pass.as_bytes(), &salt).unwrap();
    Ok(password_hash.to_string())
}

pub fn verify_password(pass: &str, hashed: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hashed) {
        Ok(h) => h,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(pass.as_bytes(), &parsed_hash)
        .is_ok()
}
