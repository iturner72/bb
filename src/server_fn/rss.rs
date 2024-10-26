use leptos::*;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssFetchProgress {
    pub company: String,
    pub status: String,
    pub new_posts: i32,
    pub skipped_posts: i32,
}

#[server(TriggerRssFetch, "/api")]
pub async fn trigger_rss_fetch() -> Result<Vec<RssFetchProgress>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        log::info!("Starting manual RSS feed fetch...");
        
        match crate::rss_service::server::process_feeds().await {
            Ok(feed_results) => {
                let progress = feed_results.into_iter()
                    .map(|result| RssFetchProgress {
                        company: result.company,
                        status: "completed".to_string(),
                        new_posts: result.new_posts,
                        skipped_posts: result.skipped_posts,
                    })
                    .collect();
                Ok(progress)
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
