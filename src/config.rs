//! Application configuration and Argon2id password hashing utilities.
//!
//! This module provides two responsibilities:
//! - [`Config`] loads runtime settings (port, password) from environment variables
//!   with sensible defaults, used by the server on startup.
//! - [`hash_password`] / [`verify_password`] wrap the Argon2id algorithm for
//!   hashing plaintext passwords and verifying them against stored hashes.

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use color_eyre::Result;
use rand::rngs::OsRng;
use std::env;

type Int = u16;

/// Server configuration loaded from environment variables.
#[derive(Debug, Clone, Default)]
pub struct Config {
    port: Int,
    password: String,
    // size:
}

impl Config {
    /// Loads config from PORT and PASSWORD env vars, falling back to defaults.
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

/// Hashes a plaintext password using Argon2id with a random salt.
pub fn hash_password(pass: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2.hash_password(pass.as_bytes(), &salt).unwrap();
    Ok(password_hash.to_string())
}

/// Verifies a plaintext password against an Argon2id hash string.
pub fn verify_password(pass: &str, hashed: &str) -> bool {
    if hashed.is_empty() {
        return true;
    }
    let parsed_hash = match PasswordHash::new(hashed) {
        Ok(h) => h,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(pass.as_bytes(), &parsed_hash)
        .is_ok()
}
