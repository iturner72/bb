// src/server_fn/rss.rs
use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssProgressUpdate {
    pub company: String,
    pub status: String,
    pub new_posts: i32,
    pub skipped_posts: i32,
    pub current_post: Option<String>, // Add title of current post being processed
}

#[server(TriggerRssFetchStream, "/api")]
pub async fn trigger_rss_fetch_stream() -> Result<Vec<RssProgressUpdate>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use futures::StreamExt;
        
        log::info!("Starting manual RSS feed fetch with streaming...");
        let (sender, mut receiver) = futures::channel::mpsc::channel(100);
        let sender = std::sync::Arc::new(std::sync::Mutex::new(sender));
        
        // Start the feed processing in a separate task
        let processing = tokio::spawn(async move {
            match crate::rss_service::server::process_feeds_with_progress(sender).await {
                Ok(_) => log::info!("Feed processing completed"),
                Err(e) => log::error!("Feed processing error: {}", e),
            }
        });

        // Collect all updates
        let mut updates = Vec::new();
        while let Some(update) = receiver.next().await {
            updates.push(update);
        }

        // Wait for processing to complete
        let _ = processing.await;
        Ok(updates)
    }

    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::ServerError("Server-side function called on client".to_string()))
    }
}
