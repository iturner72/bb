use leptos::prelude::*;
use super::types::AuthResponse;

#[server(AdminLoginFn, "/api")]
pub async fn admin_login(username: String, password: String) -> Result<AuthResponse, ServerFnError> {
    use super::types::{AuthError, to_server_error};
    #[cfg(feature = "ssr")]
    {
        use super::server::jwt;
        use http::{HeaderName, HeaderValue};

        let admin_user = std::env::var("ADMIN_USERNAME")
            .map_err(|_| AuthError::MissingEnvironmentVar("ADMIN_USERNAME".to_string()))
            .map_err(to_server_error)?;
            
        let admin_pass = std::env::var("ADMIN_PASSWORD")
            .map_err(|_| AuthError::MissingEnvironmentVar("ADMIN_PASSWORD".to_string()))
            .map_err(to_server_error)?;

        if username != admin_user || password != admin_pass {
            return Err(to_server_error(AuthError::InvalidCredentials));
        }

        let auth_response = jwt::generate_token(username).map_err(to_server_error)?;

        let auth_cookie = jwt::create_auth_cookie(&auth_response.token);

        let response_options = use_context::<leptos_axum::ResponseOptions>()
            .expect("response options not found");

        let cookie_value = HeaderValue::from_str(&auth_cookie.to_string())
            .map_err(|e| to_server_error(AuthError::CookieError(e.to_string())))?;

        response_options.insert_header(HeaderName::from_static("set-cookie"), cookie_value);

        Ok(auth_response)
    }

    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::ServerError("Server-side function called on client".to_string()))
}

#[server(LogoutFn, "/api")]
pub async fn logout() -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use super::types::{AUTH_COOKIE_NAME, AuthError, to_server_error};
        use http::{HeaderName, HeaderValue};
        use cookie::SameSite;
        use cookie::Cookie;
        use cookie::time;

        let removal_cookie = Cookie::build((AUTH_COOKIE_NAME, ""))
            .path("/")
            .secure(true)
            .http_only(true)
            .same_site(SameSite::Strict)
            .expires(time::OffsetDateTime::now_utc() - time::Duration::hours(1))
            .build();

        let response_options = use_context::<leptos_axum::ResponseOptions>()
            .expect("response options not found");

        let cookie_value = HeaderValue::from_str(&removal_cookie.to_string())
            .map_err(|e| to_server_error(AuthError::CookieError(e.to_string())))?;

        response_options.insert_header(HeaderName::from_static("set-cookie"), cookie_value);

        Ok(())
    }

    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::ServerError("Server-side function called on client".to_string()))
}

#[server(VerifyTokenFn, "/api")]
pub async fn verify_token() -> Result<bool, ServerFnError> {
    use super::types::to_server_error; 
    #[cfg(feature = "ssr")]
    {
        use super::server::jwt;
        use super::types::{AuthError, AUTH_COOKIE_NAME};
        use leptos_axum::extract;
        use axum_extra::extract::cookie::CookieJar;

        let cookie_jar = extract::<CookieJar>().await
            .map_err(|e| AuthError::CookieError(e.to_string()))
            .map_err(to_server_error)?;

        match cookie_jar.get(AUTH_COOKIE_NAME) {
            Some(cookie) => jwt::verify_token(cookie.value()).map_err(to_server_error),
            None => Ok(false)
        }
    }

    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::ServerError("Server-side function called on client".to_string()))
}
