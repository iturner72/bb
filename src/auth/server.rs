#[cfg(feature = "ssr")]
pub mod jwt {
    use super::super::types::{AuthError, AuthResponse, TokenClaims, TokenType};
    use super::super::types::{ACCESS_COOKIE_NAME, REFRESH_COOKIE_NAME};
    use super::super::secure::verify_password;
    use axum_extra::extract::cookie::{Cookie, SameSite};
    use cookie::time;
    use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
    use std::time::{SystemTime, UNIX_EPOCH};

    const ACCESS_TOKEN_DURATION: usize = 15 * 60;
    const REFRESH_TOKEN_DURATION: usize = 7 * 24 * 60 * 60;

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

    pub fn generate_tokens(username: String) -> Result<AuthResponse, AuthError> {
        let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let access_claims = TokenClaims {
            sub: username.clone(),
            exp: now + ACCESS_TOKEN_DURATION,
            iat: now,
            token_type: TokenType::Access,
        };

        let access_token = encode(
            &Header::default(),
            &access_claims,
            &EncodingKey::from_secret(jwt_secret.as_bytes())
        ).map_err(|e| AuthError::TokenCreation(e.to_string()))?;

        let refresh_claims = TokenClaims {
            sub: username,
            exp: now + REFRESH_TOKEN_DURATION,
            iat: now,
            token_type: TokenType::Refresh,
        };

        let refresh_token = encode(
            &Header::default(),
            &refresh_claims,
            &EncodingKey::from_secret(jwt_secret.as_bytes())
        ).map_err(|e| AuthError::TokenCreation(e.to_string()))?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            access_expires_in: ACCESS_TOKEN_DURATION,
            refresh_expires_in: REFRESH_TOKEN_DURATION,
        })
    }

    pub fn create_auth_cookies(auth_response: &AuthResponse) -> Vec<Cookie<'static>> {
        vec![
            Cookie::build((ACCESS_COOKIE_NAME, auth_response.access_token.clone()))
                .path("/")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Strict)
                .expires(time::OffsetDateTime::now_utc() + time::Duration::minutes(15))
                .build(),

            Cookie::build((REFRESH_COOKIE_NAME, auth_response.refresh_token.clone()))
                .path("/")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Strict)
                .expires(time::OffsetDateTime::now_utc() + time::Duration::days(7))
                .build()
        ]
    }

    pub fn verify_and_refresh_tokens(
        access_token: Option<&str>,
        refresh_token: Option<&str>,
    ) -> Result<Option<AuthResponse>, AuthError> {
        log::debug!("Token verification started");
        log::debug!("Access token present: {}", access_token.is_some());
        log::debug!("Refresh token present: {}", refresh_token.is_some());

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| AuthError::MissingEnvironmentVar("JWT_SECRET".to_string()))?;

        let validation = Validation::default();

        if let Some(token) = access_token {
            log::debug!("Verifying access token");
            match decode::<TokenClaims>(
                token,
                &DecodingKey::from_secret(jwt_secret.as_bytes()),
                &validation
            ) {
                Ok(token_data) => {
                    log::debug!("Access token decoded successfully");
                    if token_data.claims.token_type != TokenType::Access {
                        log::debug!("Invalid token type: expected Access");
                        return Err(AuthError::TokenVerification("Invalid token type".to_string()));
                    }
                    log::debug!("Access token is valid");
                    return Ok(None);
                },
                Err(e) => {
                    log::debug!("Access token verification failed: {}", e);
                    if e.kind() != &jsonwebtoken::errors::ErrorKind::ExpiredSignature {
                        return Err(AuthError::TokenVerification(e.to_string()));
                    }
                    log::debug!("Access token expired, attempting refresh");
                }
            }
        }

        if let Some(token) = refresh_token {
            log::debug!("Attempting token refresh");
            match decode::<TokenClaims>(
                token,
                &DecodingKey::from_secret(jwt_secret.as_bytes()),
                &validation
            ) {
                Ok(token_data) => {
                    log::debug!("Refresh token decoded successfully");
                    if token_data.claims.token_type != TokenType::Refresh {
                        log::debug!("Invalid token type: expected Refresh");
                        return Err(AuthError::TokenVerification("Invalid token type".to_string()));
                    }

                    log::debug!("Generating new tokens");
                    let new_tokens = generate_tokens(token_data.claims.sub)?;
                    log::debug!("New tokens generated successfully");
                    Ok(Some(new_tokens))
                },
                Err(e) => {
                    log::debug!("Refresh token verification failed: {}", e);
                    Err(AuthError::TokenVerification(e.to_string()))
                }
            }
        } else {
            log::debug!("No refresh token available");
            Err(AuthError::TokenExpired)
        }

    }
}

#[cfg(feature = "ssr")]
pub mod middleware {
    use axum::{
        body::Body,
        http::{Request, HeaderValue, header},
        middleware::Next,
        response::{Response, IntoResponse},
        http::StatusCode,
    };
    use axum_extra::extract::cookie::{CookieJar, Cookie};
    use super::super::types::{ACCESS_COOKIE_NAME, REFRESH_COOKIE_NAME};
    use super::jwt;

    pub async fn require_auth(
        cookie_jar: CookieJar,
        request: Request<Body>,
        next: Next,
    ) -> Result<Response, StatusCode> {
        log::debug!(
            "Auth middleware - Processing request to: {} {}",
            request.method(),
            request.uri()
        );

        let access_token = cookie_jar.get(ACCESS_COOKIE_NAME).map(|c| c.value().to_string());
        let refresh_token = cookie_jar.get(REFRESH_COOKIE_NAME).map(|c| c.value().to_string());

        log::debug!(
            "Auth middleware - Found tokens - Access: {}, Refresh: {}",
            access_token.is_some(),
            refresh_token.is_some()
        );

        match jwt::verify_and_refresh_tokens(
            access_token.as_deref(),
            refresh_token.as_deref(),
        ) {
            Ok(maybe_new_tokens) => {
                let mut response = next.run(request).await;

                if let Some(new_tokens) = maybe_new_tokens {
                    log::debug!("Auth middleware - Setting refreshed tokens in cookies");
                    let cookies = jwt::create_auth_cookies(&new_tokens);
                    for cookie in cookies {
                        log::debug!("Auth middleware - Setting cookie: {}", cookie.name());
                        if let Ok(cookie_value) = HeaderValue::from_str(&cookie.to_string()) {
                            response.headers_mut()
                                .append(header::SET_COOKIE, cookie_value);
                        }
                    }
                } else {
                    log::debug!("Auth middleware - Using existing valid tokens");
                }

                Ok(response)
            },
            Err(e) => {
                log::debug!("Auth middleware - Authentication failed: {:?}", e);

                let mut response = StatusCode::UNAUTHORIZED.into_response();

                log::debug!("Auth middleware - Clearing invalid cookies");
                let expired_cookies = [
                    Cookie::build((ACCESS_COOKIE_NAME, ""))
                        .path("/")
                        .expires(cookie::time::OffsetDateTime::now_utc())
                        .build(),
                    Cookie::build((REFRESH_COOKIE_NAME, ""))
                        .path("/")
                        .expires(cookie::time::OffsetDateTime::now_utc())
                        .build(),
                ];

                for cookie in expired_cookies {
                    if let Ok(cookie_value) = HeaderValue::from_str(&cookie.to_string()) {
                        response.headers_mut()
                            .append(header::SET_COOKIE, cookie_value);
                    }
                }

                Err(StatusCode::UNAUTHORIZED)
            }
        }
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Once;
    use once_cell::sync::Lazy;
    use tokio::sync::Mutex;
    use crate::auth::AuthError;

    // global mutex for environment variable operations
    static ENV_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
    static INIT: Once = Once::new();

    async fn initialize() {
        INIT.call_once(|| {
            env::set_var("JWT_SECRET", "test_secret_for_testing_only");
            env::set_var("ADMIN_USERNAME", "test_admin");
            // Test hash for password "test_password"
            env::set_var("ADMIN_PASSWORD_HASH", "JGFyZ29uMmlkJHY9MTkkbT0xOTQ1Nix0PTIscD0xJDBOM2l6OGtESkpBTVZ1T0grMnlIWEEkY0RmbjhuaUp4bjJ6SE9kbFlGVUErT2VsZmV5enJXUG1McWtXODBFVHRnYw==");
        });
    }

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

            let result = jwt::verify_token(&auth_response.token);
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
        use crate::auth::secure::verify_password;

        #[tokio::test]
        async fn test_verify_password_directly() {
            let _lock = ENV_MUTEX.lock().await;
            initialize().await;

            let stored_hash = env::var("ADMIN_PASSWORD_HASH").expect("Hash should be set");
            println!("\nTesting direct password verification:");
            println!("Stored hash: {}", stored_hash);
            
            let result = verify_password("test_password", &stored_hash);
            println!("Direct verification result: {:?}", result);
            
            assert!(result.is_ok(), "Password verification should not error");
            assert!(result.unwrap(), "Password should verify correctly");
        }

        #[tokio::test]
        async fn test_successful_login() {
            let _lock = ENV_MUTEX.lock().await;
            initialize().await;

            // First verify the environment is set up correctly
            let username = env::var("ADMIN_USERNAME").expect("Username should be set");
            let stored_hash = env::var("ADMIN_PASSWORD_HASH").expect("Hash should be set");
            println!("\nTest environment:");
            println!("Username: {}", username);
            println!("Stored hash: {}", stored_hash);

            // Try the authentication
            let result = jwt::authenticate_admin("test_admin", "test_password").await;

            match &result {
                Ok(valid) => println!("Authentication result: {}", valid),
                Err(e) => println!("Authentication error: {:?}", e),
            }

            // Try direct password verification
            let verify_result = verify_password("test_password", &stored_hash);
            println!("Direct password verification result: {:?}", verify_result);

            assert!(result.is_ok(), "Authentication should succeed");
            assert!(result.unwrap(), "Authentication should return true");
        }

        #[tokio::test]
        async fn test_failed_login_wrong_password() {
            let _lock = ENV_MUTEX.lock().await;
            initialize().await;

            let result = jwt::authenticate_admin("test_admin", "wrong_password").await;
            println!("Wrong password test result: {:?}", result);
            
            assert!(result.is_ok(), "Authentication should process without error");
            assert!(!result.unwrap(), "Authentication should return false for wrong password");
        }

        #[tokio::test]
        async fn test_missing_env_vars() {
            let _lock = ENV_MUTEX.lock().await;

            let _guard = EnvVarGuard::new(vec![
                "JWT_SECRET".to_string(),
                "ADMIN_USERNAME".to_string(),
                "ADMIN_PASSWORD_HASH".to_string(),
            ]);

            let result = jwt::authenticate_admin("test_admin", "test_password").await;
            assert!(result.is_err(), "Authentication should fail with missing env vars");

            match result {
                Err(AuthError::MissingEnvironmentVar(_)) => (),
                other => panic!("Expected MissingEnvironmentVar error, got {:?}", other),
            }
        }
    }
}
