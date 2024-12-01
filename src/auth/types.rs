use std::fmt;
use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthResponse {
    pub token: String,
    pub expires_in: usize,
}

#[derive(Debug)]
pub enum AuthError {
    TokenCreation(String),
    TokenVerification(String),
    InvalidCredentials,
    MissingEnvironmentVar(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::TokenCreation(e) => write!(f, "Failed to create token: {}", e),
            AuthError::TokenVerification(e) => write!(f, "Failed to verify token: {}", e),
            AuthError::InvalidCredentials => write!(f, "Invalid username or password"),
            AuthError::MissingEnvironmentVar(var) => write!(f, "Missing environment variable: {}", var),
        }
    }
}

pub fn to_server_error(e: AuthError) -> ServerFnError {
    ServerFnError::ServerError(e.to_string())
}
