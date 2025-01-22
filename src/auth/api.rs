use leptos::prelude::*;

#[server(AdminLoginFn, "/api")]
pub async fn admin_login(username: String, password: String) -> Result<(), ServerFnError> {
    use super::types::{AuthError, to_server_error};

    #[cfg(feature = "ssr")]
    {
        use super::server::jwt;
        use http::{HeaderName, HeaderValue};

        log::debug!("Attempting login for user: {}", username);

        match jwt::authenticate_admin(&username, &password).await {
            Ok(true) => {
                log::debug!("Authentication successful, generating tokens");
                let auth_response = jwt::generate_tokens(username)
                    .map_err(to_server_error)?;

                log::debug!("Tokens generated, creating cookies");
                let cookies = jwt::create_auth_cookies(&auth_response);
                let response_options = use_context::<leptos_axum::ResponseOptions>()
                    .expect("response options not found");

                for cookie in cookies {
                    log::debug!("Setting cookie: {}", cookie.name());
                    let cookie_value = HeaderValue::from_str(&cookie.to_string())
                        .map_err(|e| to_server_error(AuthError::CookieError(e.to_string())))?;

                    response_options.insert_header(
                        HeaderName::from_static("set-cookie"),
                        cookie_value
                    );
                }

                log::debug!("Login successful, all cookies set");
                Ok(())
            }
            Ok(false) => {
                log::debug!("Authentication failed: invalid credentials");
                Err(to_server_error(AuthError::InvalidCredentials))
            },
            Err(e) => {
                log::debug!("Authentication error: {:?}", e);
                Err(to_server_error(e))
            },
        }

    }

    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::ServerError("Server-side function called on client".to_string()))
}

#[server(LogoutFn, "/api")]
pub async fn logout() -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use super::types::{ACCESS_COOKIE_NAME, REFRESH_COOKIE_NAME, AuthError, to_server_error};
        use http::{HeaderName, HeaderValue};
        use cookie::{Cookie, SameSite};
        use cookie::time;

        let response_options = use_context::<leptos_axum::ResponseOptions>()
            .expect("response options not found");

        let expired_cookies = [
            Cookie::build((ACCESS_COOKIE_NAME, ""))
                .path("/")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Strict)
                .expires(time::OffsetDateTime::now_utc() - time::Duration::hours(1))
                .build(),

            Cookie::build((REFRESH_COOKIE_NAME, ""))
                .path("/")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Strict)
                .expires(time::OffsetDateTime::now_utc() - time::Duration::hours(1))
                .build(),
        ];


        for cookie in expired_cookies {
            let cookie_value = HeaderValue::from_str(&cookie.to_string())
                .map_err(|e| to_server_error(AuthError::CookieError(e.to_string())))?;

            response_options.insert_header(
                HeaderName::from_static("set-cookie"),
                cookie_value
            );
        }

        Ok(())
    }

    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::ServerError("Server-side function called on client".to_string()))
}

#[server(VerifyTokenFn, "/api")]
pub async fn verify_token() -> Result<bool, ServerFnError> {
    use super::types::{AuthError, to_server_error};
    
    #[cfg(feature = "ssr")]
    {
        use super::server::jwt;
        use super::types::{ACCESS_COOKIE_NAME, REFRESH_COOKIE_NAME};
        use leptos_axum::extract;
        use axum_extra::extract::cookie::CookieJar;
        use http::{HeaderName, HeaderValue};

        let cookie_jar = match extract::<CookieJar>().await {
            Ok(jar) => jar,
            Err(_) => return Ok(false),
        };

        let access_token = cookie_jar.get(ACCESS_COOKIE_NAME).map(|c| c.value().to_string());
        let refresh_token = cookie_jar.get(REFRESH_COOKIE_NAME).map(|c| c.value().to_string());

        match jwt::verify_and_refresh_tokens(
            access_token.as_deref(),
            refresh_token.as_deref(),
        ) {
            Ok(maybe_new_tokens) => {
                if let Some(new_tokens) = maybe_new_tokens {
                    let response_options = use_context::<leptos_axum::ResponseOptions>()
                        .expect("response options not found");

                    let cookies = jwt::create_auth_cookies(&new_tokens);
                    for cookie in cookies {
                        let cookie_value = HeaderValue::from_str(&cookie.to_string())
                            .map_err(|e| to_server_error(AuthError::CookieError(e.to_string())))?;

                        response_options.insert_header(
                            HeaderName::from_static("set-cookie"),
                            cookie_value
                        );
                    }
                }
                Ok(true)
            },
            Err(_) => Ok(false),
        }
    }

    #[cfg(not(feature = "ssr"))]
    Ok(false)
}
