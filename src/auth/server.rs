#[cfg(feature = "ssr")]
pub mod jwt {
    use super::super::types::{AuthError, AuthResponse, AUTH_COOKIE_NAME};
    use super::super::secure::verify_password;
    use axum_extra::extract::cookie::{Cookie, SameSite};
    use cookie::time;
    use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
    use serde::{Deserialize, Serialize};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
        iat: usize,
    }

    pub async fn authenticate_admin(username: &str, password: &str) -> Result<bool, AuthError> {
        let admin_user = std::env::var("ADMIN_USERNAME")
            .map_err(|_| AuthError::MissingEnvironmentVar("ADMIN_USERNAME".to_string()))?;
            
        let stored_hash = std::env::var("ADMIN_PASSWORD_HASH")
            .map_err(|_| AuthError::MissingEnvironmentVar("ADMIN_PASSWORD_HASH".to_string()))?;
    
        if username != admin_user {
            return Ok(false);
        }
    
        match verify_password(password, &stored_hash) {
            Ok(valid) => {
                log::info!("Password verification result: {}", valid);
                Ok(valid)
            },
            Err(e) => {
                log::error!("Password verification error: {}", e);
                Err(AuthError::TokenCreation(e))
            }
        }
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

    pub fn create_auth_cookie(token: &str) -> Cookie<'static> {
        Cookie::build((AUTH_COOKIE_NAME, token.to_owned()))
            .path("/")
            .secure(true)
            .http_only(true)
            .same_site(SameSite::Strict)
            .expires(time::OffsetDateTime::now_utc() + time::Duration::hours(1))
            .build()
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
use leptos::prelude::*;
impl crate::auth::api::AdminLoginFn {
    pub async fn run(username: String, password: String) -> Result<AuthResponse, ServerFnError> {
        log::info!("AdminLoginFn::run called with username: {}", username);
        
        match jwt::authenticate_admin(&username, &password).await {
            Ok(true) => {
                log::info!("Authentication successful");
                jwt::generate_token(username).map_err(to_server_error)
            },
            Ok(false) => {
                log::info!("Authentication failed - invalid credentials");
                Err(to_server_error(AuthError::InvalidCredentials))
            },
            Err(e) => {
                log::error!("Authentication error: {:?}", e);
                Err(to_server_error(e))
            }
        }
    }
}

#[cfg(feature = "ssr")]
impl crate::auth::api::VerifyTokenFn {
    pub async fn run(token: String) -> Result<bool, ServerFnError> {
        jwt::verify_token(&token).map_err(to_server_error)
    }
}

#[cfg(feature = "ssr")]
pub mod middleware {
    use axum::{
        body::Body,
        http::Request,
        middleware::Next,
        response::Response,
        http::StatusCode,
    };
    use axum_extra::extract::cookie::CookieJar;
    use super::super::types::AUTH_COOKIE_NAME;
    use super::jwt;

    pub async fn require_auth(
        cookie_jar: CookieJar,
        request: Request<Body>,
        next: Next,
    ) -> Result<Response, StatusCode> {
        let token = cookie_jar
            .get(AUTH_COOKIE_NAME)
            .map(|cookie| cookie.value().to_string())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        match jwt::verify_token(&token) {
            Ok(true) => {
                let response = next.run(request).await;
                Ok(response)
            },
            _ => Err(StatusCode::UNAUTHORIZED),
        }
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::auth::api::AdminLoginFn;
    use std::env;
    use std::sync::Once;
    use once_cell::sync::Lazy;
    use tokio::sync::Mutex;

    // global mutex for environment variable operations
    static ENV_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
    static INIT: Once = Once::new();

    async fn initialize() {
        INIT.call_once(|| {
            env::set_var("JWT_SECRET", "test_secret_for_testing_only");
            env::set_var("ADMIN_USERNAME", "test_admin");
            env::set_var("ADMIN_PASSWORD", "test_password");
        });
    }

    // helper to clear env vars temporarily
    struct EnvVarGuard {
        vars: Vec<String>,
        previous_values: std::collections::HashMap<String, Option<String>>,
    }

    impl EnvVarGuard {
        fn new(vars: Vec<String>) -> Self {
            let mut previous_values = std::collections::HashMap::new();
            for var in &vars {
                previous_values.insert(var.clone(), env::var(var).ok());
                env::remove_var(var);
            }
            Self { vars, previous_values }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            for var in &self.vars {
                if let Some(Some(value)) = self.previous_values.get(var) {
                    env::set_var(var, value);
                } else {
                    env::remove_var(var);
                }
            }
        }
    }

    mod jwt_tests {
        use super::*;

        #[tokio::test]
        async fn test_generate_token() {
            let _lock = ENV_MUTEX.lock().await;
            initialize().await;

            let result = jwt::generate_token("test_user".to_string());
            assert!(result.is_ok(), "Token generation should succeed");

            let auth_response = result.unwrap();
            assert!(!auth_response.token.is_empty(), "Token should not be empty");
            assert_eq!(auth_response.expires_in, 3600, "Expiration should be 3600 seconds");
        }

        #[tokio::test]
        async fn test_verify_token() {
            let _lock = ENV_MUTEX.lock().await;
            initialize().await;

            let auth_response = jwt::generate_token("test_user".to_string())
                .expect("Token generation should succeed");

            // debug failed token gen
            println!("Generated token: {}", auth_response.token);
            println!("JWT_SECRET environment variable: {:?}", env::var("JWT_SECRET"));

            let result = jwt::verify_token(&auth_response.token);

            if let Err(ref e) = result {
                println!("Verification error: {:?}", e);
            }

            assert!(result.is_ok(), "Token verification should succeed");
            assert!(result.unwrap(), "Token should be valid");
        }

        #[tokio::test]
        async fn test_verify_invalid_token() {
            let _lock = ENV_MUTEX.lock().await;
            initialize().await;

            let result = jwt::verify_token("invalid.token.here");
            assert!(result.is_err(), "Invalid token should fail verification");

            match result {
                Err(AuthError::TokenVerification(_)) => (),
                other => panic!("Expected TokenVerification error, got {:?}", other),
            }
        }
    }

    mod admin_login_tests {
        use super::*;

        #[tokio::test]
        async fn test_successful_login() {
            let _lock = ENV_MUTEX.lock().await;
            initialize().await;

            let result = AdminLoginFn::run(
                "test_admin".to_string(),
                "test_password".to_string()
            ).await;

            assert!(result.is_ok(), "Login should succeed with correct credentials");
        }

        #[tokio::test]
        async fn test_failed_login_wrong_password() {
            let _lock = ENV_MUTEX.lock().await;
            initialize().await;

            let result = AdminLoginFn::run(
                "test_admin".to_string(),
                "wrong_password".to_string()
            ).await;

            assert!(result.is_err(), "Login should fail with wrong password");
            assert!(matches!(
                result.unwrap_err(),
                ServerFnError::ServerError(e) if e.contains("Invalid username or password")
            ));
        }

        #[tokio::test]
        async fn test_missing_env_vars() {
            let _lock = ENV_MUTEX.lock().await;

            // create guard after aquiring the lock
            let _guard = EnvVarGuard::new(vec![
                "JWT_SECRET".to_string(),
                "ADMIN_USERNAME".to_string(),
                "ADMIN_PASSWORD".to_string(),
            ]);

            let result = AdminLoginFn::run(
                "test_admin".to_string(),
                "test_password".to_string()
            ).await;

            assert!(result.is_err(), "Login should fail with missing env vars");

            if let Err(ServerFnError::ServerError(e)) = result {
                assert!(e.contains("Missing environment variable"),
                    "Error should indicate missing environment variable: {}", e);
            } else {
                panic!("Unexpected error type: {:?}", result);
            }
        }
    }
}
