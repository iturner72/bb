#[cfg(feature = "ssr")]
pub mod embeddings {

use async_openai::{
    types::{CreateEmbeddingRequestArgs, EmbeddingInput},
    Client,
};
use leptos::prelude::*;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use std::{collections::HashMap, convert::Infallible};
use axum::response::sse::Event;
use std::error::Error;
use log::{info, error};

use crate::components::poasts::Poast;
use crate::server_fn::{invalidate_poasts_cache, RssProgressUpdate};

// will need to run this in supabase
const _MIGRATION_SQL: &str = r#"
CREATE TABLE post_embeddings (
    link TEXT PRIMARY KEY REFERENCES poasts(link),
    embedding vector(1536)
);
CREATE INDEX ON post_embeddings USING ivfflat (embedding vector_cosine_ops);
"#;

pub async fn generate_embeddings(
    progress_sender: tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
    cancel_token: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting embeddings generation process");
    let openai = Client::new();
    let supabase = crate::supabase::get_client();
    let mut company_states: HashMap<String, RssProgressUpdate> = HashMap::new();

    if cancel_token.is_cancelled() {
        info!("Embedding generation cancelled before starting");
        return Ok(())
    }

    let embeddings_response = supabase
        .from("post_embeddings")
        .select("link")
        .execute()
        .await?;

    let embeddings_text = embeddings_response.text().await?;
    let embeddings_value: serde_json::Value = serde_json::from_str(&embeddings_text)?;
    
    let existing_links: Vec<String> = if let serde_json::Value::Array(arr) = embeddings_value {
        arr.iter()
            .filter_map(|v| v.get("link"))
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect()
    } else {
        Vec::new()
    };

    info!("Found {} existing embeddings", existing_links.len());

    let posts_response = if !existing_links.is_empty() {
        let filter_conditions = existing_links
            .iter()
            .map(|link| format!("link.neq.{}", link))
            .collect::<Vec<_>>()
            .join(",");
    
        info!("Using filter conditions: {}", filter_conditions);
    
        supabase
            .from("poasts")
            .select("*")
            .or(&filter_conditions)
    } else {
        supabase
            .from("poasts")
            .select("*")
    };

    let response = posts_response.execute().await?;
    let response_text = response.text().await?.to_string();
    
    info!("Supabase posts response: {}", response_text);

    let posts_value: serde_json::Value = serde_json::from_str(&response_text)?;
    let posts: Vec<Poast> = if let serde_json::Value::Array(arr) = posts_value {
        arr.iter()
            .filter_map(|v| {
                let result = serde_json::from_value(v.clone());
                if let Err(ref e) = result {
                    error!("Failed to parse post: {}", e);
                }
                result.ok()
            })
            .collect()
    } else {
        return Err("Expected array response from Supabase".into());
    };

    info!("Found {} posts needing embeddings", posts.len());

    for (index, post) in posts.iter().enumerate() {

        if cancel_token.is_cancelled() {
            info!("Embeddings generation cancelled after {} posts", index);
            return Ok(());
        }

        let company_progress = company_states 
            .entry(post.company.clone())
            .or_insert(RssProgressUpdate {
                company: post.company.clone(),
                status: "processing".to_string(),
                new_posts: 0,
                skipped_posts: 0,
                current_post: Some(post.title.clone()),
            });

        info!(
            "Processing post {}/{}: '{}' from {}",
            index + 1, posts.len(), post.title, post.company
        );

        company_progress.current_post = Some(post.title.clone());
        company_progress.status = "generating embedding".to_string();

        progress_sender.send(company_progress.clone().into_event())
            .await
            .map_err(|e| format!("Failed to send progress update: {}", e))?;

        let text = format!(
            "{}\n{}\n{}",
            post.title,
            post.summary.as_deref().unwrap_or(""),
            post.full_text.as_deref().unwrap_or(""),
        );

        match openai
            .embeddings()
            .create(CreateEmbeddingRequestArgs::default()
                .model("text-embedding-3-small")
                .input(EmbeddingInput::String(text))
                .build()?)
            .await
        {
            Ok(embedding_response) => {
                company_progress.status = "storing".to_string();
                progress_sender.send(company_progress.clone().into_event())
                    .await
                    .map_err(|e| format!("Failed to send progress update: {}", e))?;

                match supabase
                    .from("post_embeddings")
                    .insert(serde_json::to_string(&json!({
                        "link": post.link,
                        "embedding": embedding_response.data[0].embedding
                    }))?)
                    .execute()
                    .await
                {
                    Ok(_) => {
                        company_progress.new_posts += 1;
                        info!("Successfully stored embedding for '{}'", post.title);
                    },
                    Err(e) => {
                        error!("Failed to store embedding for '{}': {}", post.title, e);
                        company_progress.skipped_posts += 1;
                    }
                }
            },
            Err(e) => {
                error!("Failed to generate embedding for '{}': {}", post.title, e);
                company_progress.skipped_posts += 1;
            }
        }

        progress_sender.send(company_progress.clone().into_event())
            .await
            .map_err(|e| format!("Failed to send progress update: {}", e))?;

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    for (_, progress) in company_states.iter_mut() {
        progress.status = "completed".to_string();
        progress.current_post = None;
        progress_sender.send(progress.clone().into_event())
            .await
            .map_err(|e| format!("Failed to send final progress update: {}", e))?;
    }

    invalidate_poasts_cache().await.map_err(|e| {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn Error + Send + Sync>
    })?;

    progress_sender
        .send(Ok(Event::default().data("[DONE]")))
        .await
        .map_err(|e| format!("Failed to send completion signal: {}", e))?;

    info!("Embeddings generation completed successfully");
    Ok(())
} 

#[cfg(feature = "ssr")]
// code smell will call this with an CLI binary (first time i've worked with vector extension in
// supabase, it worked first try) will remove this, or consider alternative methods for calling.
//
// I might keep this for testing new models later.
pub async fn test_single_embedding() -> Result<(), Box<dyn std::error::Error>> {
    use async_openai::{
        types::{CreateEmbeddingRequestArgs, EmbeddingInput},
        Client,
    };
    use crate::components::poasts::Poast;
    
    let openai = Client::new();
    let supabase = crate::supabase::get_client();

    log::info!("Fetching a post from Supabase...");
    let response = supabase
        .from("poasts")
        .select("*")
        .limit(1)
        .execute()
        .await?;

    let response_text = response.text().await?;
    log::debug!("Supabase response: {}", response_text);

    let posts: Vec<Poast> = serde_json::from_str(&response_text)?;
    
    if let Some(post) = posts.first() {
        log::info!("Processing post: {}", post.title);
        
        let text = format!(
            "{}\n{}\n{}",
            post.title,
            post.summary.as_deref().unwrap_or(""),
            post.full_text.as_deref().unwrap_or("")
        );

        log::info!("Getting embedding from OpenAI");
        let embedding_response = openai
            .embeddings()
            .create(CreateEmbeddingRequestArgs::default()
                .model("text-embedding-3-small")
                .input(EmbeddingInput::String(text))
                .build()?)
            .await?;

        let embedding = embedding_response.data[0].embedding.clone();
        log::info!("Got embedding with {} dimensions", embedding.len());

        log::info!("Inserting embedding into Supabase");
        let insert_data = serde_json::json!({
            "link": post.link,
            "embedding": embedding
        });
        
        log::debug!("Insert data: {}", insert_data);
        
        let result = supabase
            .from("post_embeddings")
            .insert(insert_data.to_string())
            .execute()
            .await?;

        log::info!("Insertion result status: {}", result.status());
        log::debug!("Insertion response: {}", result.text().await?);
    } else {
        log::info!("No posts found!");
    }

    Ok(())
}

}
