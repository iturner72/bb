use leptos::prelude::*;
use super::types::AuthResponse;

#[server(AdminLoginFn, "/api")]
pub async fn admin_login(username: String, password: String) -> Result<AuthResponse, ServerFnError> {
    use super::types::{AuthError, to_server_error};
    #[cfg(feature = "ssr")]
    {
        use super::server::jwt;
        let admin_user = std::env::var("ADMIN_USERNAME")
            .map_err(|_| AuthError::MissingEnvironmentVar("ADMIN_USERNAME".to_string()))
            .map_err(to_server_error)?;
            
        let admin_pass = std::env::var("ADMIN_PASSWORD")
            .map_err(|_| AuthError::MissingEnvironmentVar("ADMIN_PASSWORD".to_string()))
            .map_err(to_server_error)?;

        if username != admin_user || password != admin_pass {
            return Err(to_server_error(AuthError::InvalidCredentials));
        }

        jwt::generate_token(username).map_err(to_server_error)
    }

    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::ServerError("Server-side function called on client".to_string()))
}

#[server(VerifyTokenFn, "/api")]
pub async fn verify_token(token: String) -> Result<bool, ServerFnError> {
    use super::types::to_server_error; 
    #[cfg(feature = "ssr")]
    {
        use super::server::jwt;
        jwt::verify_token(&token).map_err(to_server_error)
    }

    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::ServerError("Server-side function called on client".to_string()))
}
