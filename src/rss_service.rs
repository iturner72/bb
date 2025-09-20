#[cfg(feature = "ssr")]
pub mod server {
use feed_rs::parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use std::error::Error;
use std::convert::Infallible;
use axum::response::sse::Event;
use chrono::{DateTime, Utc, Duration};
use serde_json::json;
use async_openai::{
    config::OpenAIConfig,
    types::{
        CreateChatCompletionRequest,
        ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage,
        ResponseFormat,
    },
    Client,
};

use crate::server_fn::{invalidate_poasts_cache, RssProgressUpdate};

const STANDARD_ENTRY_LIMIT: usize = 10;
const EXTENDED_ENTRY_LIMIT: usize = 20;
const EXTENDED_PROCESSING_THRESHOLD: Duration = Duration::days(14);

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedResult {
    pub company: String,
    pub new_posts: i32,
    pub skipped_posts: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedLink {
    company: String,
    link: String,
    last_processed: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostInsights {
    pub summary: String,
    pub buzzwords: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlogPost {
    published_at: String,
    company: String,
    title: String,
    link: String,
    description: Option<String>,
    summary: Option<String>,
    full_text: Option<String>,
    buzzwords: Option<Vec<String>>,
}

#[derive(Debug, thiserror::Error)]
    pub enum InsightError {
        #[error("OpenAI error: {0}")]
        OpenAI(String),

        #[error("JSON error: {0}")]
        Json(#[from] serde_json::Error),
    }

pub async fn process_feeds_with_progress(
    progress_sender: tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
    cancel_token: CancellationToken,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let supabase = crate::supabase::get_client();
    let openai = Client::with_config(OpenAIConfig::default());
    
    let response = supabase
        .from("links")
        .select("company,link,last_processed")
        .execute()
        .await?;
    
    let feed_links: Vec<FeedLink> = serde_json::from_str(&response.text().await?)?;
    
    for link_info in feed_links {
        // check for cancellation before processing each feed
        if cancel_token.is_cancelled() {
            log::info!("RSS processing cancelled");
            return Ok(());
        }

        let mut company_progress = RssProgressUpdate {
            company: link_info.company.clone(),
            status: "processing".to_string(),
            new_posts: 0,
            skipped_posts: 0,
            current_post: None,
        };
        
        // Send initial company status
        progress_sender.send(
            company_progress.clone().into_event()
        ).await?;
        
        // Process feed entries
        let response = match reqwest::Client::new()
            .get(&link_info.link)
            .header("User-Agent", "Mozilla/5.0 (compatible; BlogBot/1.0)")
            .send()
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    log::warn!("Skipping {} - Failed to fetch feed: {}", link_info.company, e);
                    company_progress.status = "skipped".to_string();
                    progress_sender.send(company_progress.into_event()).await?;
                    continue;
                }
            };
            
        let content = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                log::warn!("Skipping {} - Failed to read feed content: {}", link_info.company, e);
                company_progress.status = "skipped".to_string();
                progress_sender.send(company_progress.into_event()).await?;
                continue;
            }
        };

        let feed = match parser::parse(&content[..]) {
            Ok(parsed_feed) => parsed_feed,
            Err(e) => {
                log::warn!("Skipping {} - Failed to parse feed: {}", link_info.company, e);
                company_progress.status = "skipped".to_string();
                progress_sender.send(company_progress.into_event()).await?;
                continue;
            }
        };

        // sort by date, reverse chron 
        let mut entries = feed.entries;
        entries.sort_by(|a, b| b.published.cmp(&a.published)); 

        // determine how many entries to process based on last_processed timestamp
        let entries_limit = match &link_info.last_processed {
            None => {
                log::info!("First time processing {}, fetching all entries", link_info.company);
                entries.len()  // for new companies, process all entries
            }
            Some(last_processed) => {
                let time_since_last_process = Utc::now() - *last_processed;
                if time_since_last_process > EXTENDED_PROCESSING_THRESHOLD {
                    log::info!(
                        "Over 2 weeks since last processing {} ({}), fetching {} entries",
                        link_info.company,
                        time_since_last_process.num_days(),
                        EXTENDED_ENTRY_LIMIT
                    );
                    EXTENDED_ENTRY_LIMIT
                } else {
                    log::info!(
                        "Recent processing for {} ({} days ago), fetching {} entries",
                        link_info.company,
                        time_since_last_process.num_days(),
                        STANDARD_ENTRY_LIMIT
                    );
                    STANDARD_ENTRY_LIMIT
                }
            }
        };

        entries.truncate(entries_limit);

        for entry in entries {
            // check for cancellation before processing each entry
            if cancel_token.is_cancelled() {
                log::info!("RSS processing cancelled while processing entries");
                return Ok(());
            }

            let title = entry.title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_else(|| "Untitled".to_string());

            company_progress.current_post = Some(title.clone());
            progress_sender.send(company_progress.clone().into_event()).await?;
            
            let entry_link = entry.links
                .first()
                .map(|l| l.href.clone())
                .unwrap_or_else(String::new);
            
            let existing = supabase
                .from("poasts")
                .select("*")
                .eq("link", &entry_link)
                .execute()
                .await?;
            
            let existing_json: Value = serde_json::from_str(&existing.text().await?)?;
            if !existing_json.as_array().unwrap_or(&Vec::new()).is_empty() {
                log::info!("Skipping existing post: {} from {}", title, link_info.company);
                company_progress.skipped_posts += 1;
                progress_sender.send(company_progress.clone().into_event()).await?;
                continue;
            }

            // Process new post...
            log::info!("Processing new post: {} from {}", title, link_info.company);
            let full_text = scrape_page(&entry_link).await?;
            let insights = get_post_insights(&openai, &title, &full_text).await?;
            
            // Insert post...
            let post = BlogPost {
                published_at: entry.published
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
                company: link_info.company.clone(),
                title: title.clone(),
                link: entry_link,
                description: entry.summary.map(|s| s.content),
                summary: Some(insights.summary),
                full_text: Some(full_text),
                buzzwords: Some(insights.buzzwords),
            };

            let post_json = serde_json::to_string(&post)?;
            supabase
                .from("poasts")
                .insert(post_json)
                .execute()
                .await?;

            company_progress.new_posts += 1;
            progress_sender.send(company_progress.clone().into_event()).await?;
        }

        // update last_processed timestamp
        let now = Utc::now();
        supabase
            .from("links")
            .update(serde_json::to_string(&json!({ "last_processed": now }))?)
            .eq("company", &link_info.company)
            .execute()
            .await?;
        
        // Send final company status
        company_progress.status = "completed".to_string();
        company_progress.current_post = None;
        progress_sender.send(company_progress.into_event()).await?;
    }

    // invalidate poasts cache to see new results immediately
    invalidate_poasts_cache().await.map_err(|e| {
        Box::new(std::io::Error::other(e.to_string())) as Box<dyn Error + Send + Sync>
    })?;
    
    progress_sender
        .send(Ok(Event::default().data("[DONE]")))
        .await?;

    log::info!("RSS processing completed, sent [DONE] message");
    Ok(())
}

pub async fn scrape_page(url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let response = reqwest::get(url).await?.text().await?;

    let document = scraper::Html::parse_document(&response);

    let main_content_selectors = [
        "article",
        ".post-content",
        ".entry-content",
        ".content",
        "main",
        ".post",
        ".article-content"
    ];

    let mut extracted_text = String::new();

    for selector_str in &main_content_selectors {
        if let Ok(selector) = scraper::Selector::parse(selector_str) {
            let content: String = document
                .select(&selector)
                .flat_map(|element| element.text())
                .collect::<Vec<_>>()
                .join(" ");

            if !content.trim().is_empty() && content.len() > extracted_text.len() {
                extracted_text = content;
            }
        }
    }

    if extracted_text.trim().is_empty() {
        let body_selector = scraper::Selector::parse("body").unwrap();
        extracted_text = document
            .select(&body_selector)
            .flat_map(|element| element.text())
            .collect();
    }

    let cleaned_text = extracted_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // truncate to stay under openai limit for gpt-4o-mini
    const MAX_CHARS: usize = 120_000; // conservative limit ~25k-40k tokens

    let final_text = if cleaned_text.len() > MAX_CHARS {
        log::info!("truncating content from {} to {} chars for URL: {}",
            cleaned_text.len(), MAX_CHARS, url);

        let truncated = &cleaned_text[..MAX_CHARS];
        match truncated.rfind('.') {
            Some(last_period) if last_period > MAX_CHARS / 2 => {
                &cleaned_text[..last_period + 1]
            }
            _ => {
                // fallback to word boundary
                match truncated.rfind(' ') {
                    Some(last_space) => &cleaned_text[..last_space],
                    None => truncated
                }
            }
        }
    } else {
        &cleaned_text
    };

    log::debug!("scraped {} characters from {}", final_text.len(), url);
    Ok(final_text.to_string())
}

pub async fn get_post_insights(
    client: &Client<OpenAIConfig>,
    title: &str,
    full_text: &str,
) -> Result<PostInsights, InsightError> {
    let system_message = ChatCompletionRequestSystemMessage {
        content: "You are a helpful assistant that creates detailed technical summaries.".into(),
        name: None,
    };

    let user_message = ChatCompletionRequestUserMessage {
        content: format!(
            "Create a detailed technical summary for a blog post based on the title and full text I provide you. \
            Include key technical details, architectures discussed, guests featured (if applicable), and main takeaways. \
            Respond only in JSON with exactly two fields:\n\
            1. 'summary': A single string containing a comprehensive summary\n\
            2. 'buzzwords': An array of 5 key technical terms or concepts\n\n\
            Title: '{}'\n\nFull Text: '{}'",
            title, full_text
        ).into(),
        name: None,
    };

    let request = CreateChatCompletionRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![
            system_message.into(),
            user_message.into(),
        ],
        response_format: Some(ResponseFormat::JsonObject),
        max_completion_tokens: Some(400),
        ..Default::default()
    };

    let response = client.chat().create(request)
        .await
        .map_err(|e| InsightError::OpenAI(e.to_string()))?;

    let content = response.choices[0].message.content.clone().unwrap_or_default();

    log::debug!("Raw OpenAI response for '{}': {}", title, content);

    match serde_json::from_str::<PostInsights>(&content) {
        Ok(insights) => Ok(insights),
        Err(e) => {
            log::error!("Failed to parse JSON for '{}': {}\nRaw content: {}", title, e, content);
            Err(InsightError::Json(e))
        }
    }
}
}

#[cfg(not(feature = "ssr"))]
pub mod server {}
