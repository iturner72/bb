use diesel::{ConnectionError, ConnectionResult};
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::ManagerConfig;
use diesel_async::AsyncPgConnection;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use rustls::ClientConfig;
use rustls_platform_verifier::ConfigVerifierExt;

pub type DbPool = Pool<AsyncPgConnection>;

pub fn establish_connection(database_url: &str) -> Result<DbPool, Box<dyn std::error::Error>> {
    let mut config = ManagerConfig::default();
    config.custom_setup = Box::new(establish_connection_with_tls);

    let mgr =
        AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(database_url, config);

    let pool = Pool::builder(mgr).max_size(8).build()?;

    Ok(pool)
}

fn establish_connection_with_tls(config: &str) -> BoxFuture<'_, ConnectionResult<AsyncPgConnection>> {
    let fut = async {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let rustls_config = ClientConfig::with_platform_verifier()
            .map_err(|e| ConnectionError::BadConnection(format!("TLS config error: {}", e)))?;
        let tls = tokio_postgres_rustls::MakeRustlsConnect::new(rustls_config);

        // rustls does not support channel binding
        let config_with_no_channel_binding = config.replace("channel_binding=require", "channel_binding=disable");
        let (client, conn) = tokio_postgres::connect(&config_with_no_channel_binding, tls)
            .await
            .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;

        AsyncPgConnection::try_from_client_and_connection(client, conn).await
    };
    fut.boxed()
}
