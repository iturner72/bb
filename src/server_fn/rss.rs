use leptos::*;

#[cfg(feature = "ssr")]
use crate::rss_service::server::process_feeds;

#[server(TriggerRssFetch, "/api")]
pub async fn trigger_rss_fetch() -> Result<String, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        log::info!("Starting manual RSS feed fetch...");
        
        match process_feeds().await {
            Ok(_) => {
                log::info!("RSS feed fetch completed successfully");
                Ok("RSS feeds processed successfully".to_string())
            },
            Err(e) => {
                log::error!("Error processing RSS feeds: {}", e);
                Err(ServerFnError::ServerError(format!("Failed to process RSS feeds: {}", e)))
            }
        }
    }

    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::ServerError("Server-side function called on client".to_string()))
    }
}
