#[cfg(feature = "ssr")]
pub mod backfill {
    use axum::response::sse::Event;
    use serde_json::Value;
    use std::convert::Infallible;
    use async_openai::{
        config::OpenAIConfig,
        Client,
    };
    use log::{info, error, warn};
    use thiserror::Error;
    use serde::{Serialize, Deserialize};

    use crate::server_fn::RssProgressUpdate;
    use crate::rss_service::server::{get_post_insights, scrape_page};

    #[derive(Serialize, Deserialize)]
    struct PostUpdate {
        summary: String,
        buzzwords: Vec<String>,
        full_text: String,
    }

    #[derive(Error, Debug)]
    pub enum BackfillError {
        #[error("Supabase error: {0}")]
        Supabase(String),
        
        #[error("OpenAI error: {0}")]
        OpenAI(String),
        
        #[error("JSON error: {0}")]
        Json(#[from] serde_json::Error),
        
        #[error("Send error: {0}")]
        Send(String),
        
        #[error("Scraping error: {0}")]
        Scrape(String),
    }

    const MAX_TOKENS_APPROX: usize = 3000;

    fn truncate_text(text: &str) -> String {
        let cleaned = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        let char_limit = MAX_TOKENS_APPROX * 4;

        if cleaned.chars().count() <= char_limit {
            cleaned
        } else {
            // take the first and last portions to capture intro and conclusion
            let portion_size = char_limit / 2;
            let start = cleaned.chars()
                .take(portion_size)
                .collect::<String>();

            let end = cleaned.chars()
                .skip(cleaned.chars().count() - portion_size)
                .collect::<String>();

            format!("{}... [text truncated] ...{}", start, end) 
        }
    }

    pub async fn backfill_missing_data(
        progress_sender: tokio::sync::mpsc::Sender<Result<Event, Infallible>>
    ) -> Result<(), BackfillError> {
        info!("Starting backfill process for posts with missing data");
        let supabase = crate::supabase::get_client();
        let openai = Client::with_config(OpenAIConfig::default());

        // find all post swith missing summaries or buzzwords
        info!("Querying Supabase for posts with missing summaries or buzzwords");
        let response = supabase
            .from("poasts")
            .select("*")
            .or("summary.is.null,buzzwords.is.null")
            .execute()
            .await
            .map_err(|e| BackfillError::Supabase(e.to_string()))?;

        let posts_text = response.text()
            .await
            .map_err(|e| BackfillError::Supabase(e.to_string()))?;

        let posts: Vec<Value> = serde_json::from_str(&posts_text)?;

        info!("Found {} posts that need backfilling", posts.len());

        for (index, post) in posts.iter().enumerate() {
            let title = post["title"].as_str().unwrap_or("").to_string();
            let link = post["link"].as_str().unwrap_or("").to_string();
            let company = post["company"].as_str().unwrap_or("").to_string();

            info!("Processing post {}/{}: '{}' from {}", index + 1, posts.len(), title, company);

            let mut company_progress = RssProgressUpdate {
                company: company.clone(), 
                status: "backfilling".to_string(),
                new_posts: 0,
                skipped_posts: 0,
                current_post: Some(title.clone()),
            };

            progress_sender.send(company_progress.clone().into_event())
                .await
                .map_err(|e| BackfillError::Send(e.to_string()))?;

            let full_text = match post["full_text"].as_str() {
                Some(text) => {
                    info!("Using existing full_text for post '{}'", title);
                    text.to_string()
                }
                None => {
                    info!("Scraping full_text for post '{}' from {}", title, link);
                    company_progress.status = "scraping".to_string();
                    progress_sender.send(company_progress.clone().into_event())
                        .await
                        .map_err(|e| BackfillError::Send(e.to_string()))?;

                    scrape_page(&link)
                        .await
                        .map_err(|e| BackfillError::Scrape(e.to_string()))?
                }
            };

            let truncated_text = truncate_text(&full_text);
            info!(
                "Text length before/after truncation: {}/{} chars",
                full_text.len(),
                truncated_text.len()
            );

            info!("Getting post insights from OpenAI for '{}'", title);
            company_progress.status = "analyzing".to_string();
            progress_sender.send(company_progress.clone().into_event())
                .await
                .map_err(|e| BackfillError::Send(e.to_string()))?;

            match get_post_insights(&openai, &title, &full_text)
                .await
                .map_err(|e| BackfillError::OpenAI(e.to_string()))
            {
                Ok(insights) => {
                    info!("Successfully got insights for '{}'", title);

                    company_progress.status = "updating".to_string();
                    progress_sender.send(company_progress.clone().into_event())
                        .await
                        .map_err(|e| BackfillError::Send(e.to_string()))?;

                    let update = PostUpdate {
                        summary: insights.summary,
                        buzzwords: insights.buzzwords,
                        full_text,
                    };

                    info!("Updating post '{}' in Supabase", title);
                    let update_json = serde_json::to_string(&update)
                        .map_err(BackfillError::Json)?;

                    match supabase
                        .from("poasts")
                        .update(&update_json)
                        .eq("link", link)
                        .execute()
                        .await
                    {
                        Ok(_) => {
                            info!("Successfully updated post '{}' in database", title);
                            company_progress.new_posts += 1;
                            company_progress.status = "completed".to_string();
                            progress_sender.send(company_progress.clone().into_event())
                                .await
                                .map_err(|e| BackfillError::Send(e.to_string()))?;
                        },
                        Err(e) => {
                            error!("Failed to update post '{}' in database: {}", title, e);
                            company_progress.skipped_posts += 1;
                            company_progress.status = "failed".to_string();
                            progress_sender.send(company_progress.clone().into_event())
                                .await
                                .map_err(|e| BackfillError::Send(e.to_string()))?;
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to get insights for post '{}': {}", title, e);
                    company_progress.skipped_posts += 1;
                    company_progress.status = "failed".to_string();
                    progress_sender.send(company_progress.into_event())
                        .await
                        .map_err(|e| BackfillError::Send(e.to_string()))?;
                }
            }

            // add small delay between poasts to help w rate limiting
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        if posts.is_empty() {
            warn!("No posts found that need backfilling");
        } else {
            info!("Backfill process completed. Processed {} posts", posts.len());
        } 

        progress_sender
            .send(Ok(Event::default().data("[DONE]")))
            .await
            .map_err(|e| BackfillError::Send(e.to_string()))?;

        Ok(())
    }
}
