#[cfg(feature = "ssr")]
pub mod refresh {
    use axum::response::sse::Event;
    use serde_json::Value;
    use tokio_util::sync::CancellationToken;
    use std::collections::HashMap;
    use std::convert::Infallible;
    use async_openai::{
        config::OpenAIConfig,
        Client,
    };
    use log::{info, error};
    use thiserror::Error;
    use serde::{Serialize, Deserialize};

    use crate::server_fn::RssProgressUpdate;
    use crate::rss_service::server::get_post_insights;

    #[derive(Error, Debug)]
    pub enum RefreshError {
        #[error("Supabase error: {0}")]
        Supabase(String),

        #[error("OpenAI error: {0}")]
        OpenAI(String),

        #[error("JSON error: {0}")]
        Json(#[from] serde_json::Error),

        #[error("Send error: {0}")]
        Send(String),
    }

    #[derive(Serialize, Deserialize)]
    struct SummaryUpdate {
        summary: String,
        buzzwords: Vec<String>,
    }

    pub async fn refresh_summaries(
        progress_sender: tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
        cancel_token: CancellationToken,
        company: Option<String>,
        start_year: Option<i32>,
        end_year: Option<i32>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let date_range_str = match (start_year, end_year) {
            (Some(start), Some(end)) => format!("years {} to {}", start, end),
            (Some(start), None) => format!("year {} onwards", start),
            (None, Some(end)) => format!("years up to {}", end),
            (None, None) => "all years".to_string(),
        };
        info!("Starting summary refresh process for {}", date_range_str);

        let supabase = crate::supabase::get_client();
        let openai = Client::with_config(OpenAIConfig::default());
        let mut company_states: HashMap<String, RssProgressUpdate> = HashMap::new();

        // Check for cancellation before starting
        if cancel_token.is_cancelled() {
            info!("Summary refresh cancelled before starting");
            return Ok(());
        }

        let mut request = supabase.from("poasts").select("*");

        if let Some(start) = start_year {
            let start_date = format!("{}-01-01", start);
            request = request.gte("published_at", start_date);
        }

        if let Some(end) = end_year {
            let end_date = format!("{}-12-31", end);
            request = request.lte("published_at", end_date);
        }

        if let Some(company_name) = &company {
            info!("Filtering for company: {}", company_name);
            request = request.eq("company", company_name);
        }

        let response = request
            .execute()
            .await
            .map_err(|e| RefreshError::Supabase(e.to_string()))?;

        let posts: Vec<Value> = serde_json::from_str(
            &response.text().await.map_err(|e| RefreshError::Supabase(e.to_string()))?
        )?;

        info!("Found {} posts to refresh", posts.len());

        for (index, post) in posts.iter().enumerate() {
            // check for cancellation
            if cancel_token.is_cancelled() {
                info!("Summary refresh cancelled after {} posts", index);
                return Ok(());
            }

            let title = post["title"].as_str().unwrap_or("").to_string();
            let company = post["company"].as_str().unwrap_or("").to_string();
            let published_at = post["published_at"].as_str().unwrap_or("").to_string();
            let full_text = post["full_text"].as_str().unwrap_or("").to_string();

            info!(
                "Processing post {}/{}: '{}' from {} (published: {})",
                index + 1, posts.len(), title, company, published_at
            );

            let company_progress = company_states.entry(company.clone()).or_insert(RssProgressUpdate {
                company: company.clone(),
                status: "refreshing".to_string(),
                new_posts: 0,
                skipped_posts: 0,
                current_post: Some(title.clone()),
            });

            company_progress.current_post = Some(format!("{} ({})", title, published_at));

            // use try_send for progress updates
            if let Err(err) = progress_sender.try_send(company_progress.clone().into_event()) {
                match err {
                    tokio::sync::mpsc::error::TrySendError::Full(_) => {
                        // channel is full, might want to wait and retry
                        info!("Progress channel is full, continuing without update");
                    }
                    tokio::sync::mpsc::error::TrySendError::Closed(_) => {
                        info!("Channel closed, stopping summary refresh");
                        return Ok(());
                    }
                }
            }

            match get_post_insights(&openai, &title, &full_text)
                .await
                .map_err(|e| RefreshError::OpenAI(e.to_string()))
            {
                Ok(insights) => {
                    company_progress.status = "updating".to_string();
                    progress_sender.send(company_progress.clone().into_event())
                        .await
                        .map_err(|e| RefreshError::Send(e.to_string()))?;

                    let update = SummaryUpdate {
                        summary: insights.summary,
                        buzzwords: insights.buzzwords,
                    };

                    match supabase
                        .from("poasts")
                        .update(&serde_json::to_string(&update)?)
                        .eq("title", &title)
                        .eq("company", &company)
                        .execute()
                        .await
                    {
                        Ok(_) => {
                            company_progress.new_posts += 1;
                            company_progress.status = "completed".to_string();
                            info!("Successfully updated post '{}' from {}", title, company);
                        }
                        Err(e) => {
                            error!("Failed to update post '{}': {}", title, e);
                            company_progress.skipped_posts += 1;
                            company_progress.status = "failed".to_string();
                        }
                    }

                    // send final progress update for this post
                    if let Err(err) = progress_sender.try_send(company_progress.clone().into_event()) {
                        match err {
                            tokio::sync::mpsc::error::TrySendError::Full(_) => {
                                info!("Progress channel is full after processing post");
                            }
                            tokio::sync::mpsc::error::TrySendError::Closed(_) => {
                                info!("Channel closed after processing post, stopping summary refresh");
                                return Ok(());
                            }
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to get insights for post '{}': {}", title, e);
                    company_progress.skipped_posts += 1;
                    company_progress.status = "failed".to_string();
                    progress_sender.send(company_progress.clone().into_event())
                        .await
                        .map_err(|e| RefreshError::Send(e.to_string()))?;
                }
            }

            // rate limiting delay
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            if cancel_token.is_cancelled() {
                info!("Refresh process cancelled after processing post");
                return Ok(());
            }
        }

        for (_, progress) in company_states.iter_mut() {
            progress.status = "completed".to_string();
            progress.current_post = None;
            progress_sender.send(progress.clone().into_event())
                .await
                .map_err(|e| RefreshError::Send(e.to_string()))?;
        }

        let _ = progress_sender.try_send(Ok(Event::default().data("[DONE]")));
    
        info!("Summary refresh completed successfully");
        Ok(())
    }
}
