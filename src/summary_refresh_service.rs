#[cfg(feature = "ssr")]
pub mod refresh {
    use axum::response::sse::Event;
    use serde_json::Value;
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
        company: Option<String>
    ) -> Result<(), RefreshError> {
        info!("Starting summary refresh process");
        let supabase = crate::supabase::get_client();
        let openai = Client::with_config(OpenAIConfig::default());
        let mut company_states: HashMap<String, RssProgressUpdate> = HashMap::new();

        let mut request = supabase.from("poasts").select("*");

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
            let title = post["title"].as_str().unwrap_or("").to_string();
            let company = post["company"].as_str().unwrap_or("").to_string();
            let full_text = post["full_text"].as_str().unwrap_or("").to_string();

            info!("Processing post {}/{}: '{}' from {}", index + 1, posts.len(), title, company);

            let company_progress = company_states.entry(company.clone()).or_insert(RssProgressUpdate {
                company: company.clone(),
                status: "refreshing".to_string(),
                new_posts: 0,
                skipped_posts: 0,
                current_post: Some(title.clone()),
            });

            company_progress.current_post = Some(title.clone());
            progress_sender.send(company_progress.clone().into_event())
                .await
                .map_err(|e| RefreshError::Send(e.to_string()))?;

            match get_post_insights(&openai, &title, &full_text)
                .await
                .map_err(|e| RefreshError::OpenAI(e.to_string()))
            {
                Ok(insights) => {
                    let update = SummaryUpdate {
                        summary: insights.summary,
                        buzzwords: insights.buzzwords,
                    };

                    match supabase
                        .from("poasts")
                        .update(&serde_json::to_string(&update)?)
                        .eq("title", &title)
                        .execute()
                        .await
                    {
                        Ok(_) => {
                            company_progress.new_posts += 1;
                            company_progress.status = "completed".to_string();
                        }
                        Err(e) => {
                            error!("Failed to update post '{}': {}", title, e);
                            company_progress.skipped_posts += 1;
                            company_progress.status = "failed".to_string();
                        }
                    }

                    progress_sender.send(company_progress.clone().into_event())
                        .await
                        .map_err(|e| RefreshError::Send(e.to_string()))?;
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
        }

        progress_sender
            .send(Ok(Event::default().data("[DONE]")))
            .await
            .map_err(|e| RefreshError::Send(e.to_string()))?;

        Ok(())
    }
}
