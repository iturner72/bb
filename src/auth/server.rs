#[cfg(feature = "ssr")]
pub mod jwt {
    use super::super::types::{AuthError, AuthResponse};
    use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
    use serde::{Deserialize, Serialize};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
        iat: usize,
    }

    pub fn generate_token(username: String) -> Result<AuthResponse, AuthError> {
        let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let expires_in = 3600;

        let claims = Claims {
            sub: username,
            exp: now + expires_in,
            iat: now,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_bytes())
        ).map_err(|e| AuthError::TokenCreation(e.to_string()))?;

        Ok(AuthResponse { token, expires_in })
    }

    pub fn verify_token(token: &str) -> Result<bool, AuthError> {
        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| AuthError::MissingEnvironmentVar("JWT_SECRET".to_string()))?;

        let validation = Validation::default();

        match decode::<Claims>(
            token,
            &DecodingKey::from_secret(jwt_secret.as_bytes()),
            &validation
        ) {
            Ok(_) => Ok(true),
            Err(e) => match e.kind() {
                &jsonwebtoken::errors::ErrorKind::ExpiredSignature => Ok(false),
                _ => Err(AuthError::TokenVerification(e.to_string()))
            }
        }
    }
}

#[cfg(feature = "ssr")]
use super::types::{AuthResponse, AuthError, to_server_error};
use leptos::*;
impl crate::auth::api::AdminLoginFn {
    pub async fn run(username: String, password: String) -> Result<AuthResponse, ServerFnError> {
        let admin_user = std::env::var("ADMIN_USERNAME")
            .map_err(|_| AuthError::MissingEnvironmentVar("ADMIN_USERNAME".to_string()))
            .map_err(to_server_error)?;
            
        let admin_pass = std::env::var("ADMIN_PASSWORD")
            .map_err(|_| AuthError::MissingEnvironmentVar("ADMIN_PASSWORD".to_string()))
            .map_err(to_server_error)?;

        if username != admin_user || password != admin_pass {
            return Err(AuthError::InvalidCredentials).map_err(to_server_error);
        }

        jwt::generate_token(username).map_err(to_server_error)
    }
}

#[cfg(feature = "ssr")]
impl crate::auth::api::VerifyTokenFn {
    pub async fn run(token: String) -> Result<bool, ServerFnError> {
        jwt::verify_token(&token).map_err(to_server_error)
    }
}
