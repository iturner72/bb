#[cfg(feature = "ssr")]
pub mod embeddings {

use async_openai::{
    types::{CreateEmbeddingRequestArgs, EmbeddingInput},
    Client,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::sync::CancellationToken;
use std::{collections::HashMap, convert::Infallible};
use axum::response::sse::Event;
use std::error::Error;
use log::{info, error};

use crate::components::poasts::Poast;
use crate::server_fn::{invalidate_poasts_cache, RssProgressUpdate};

#[derive(Debug, Serialize, Deserialize)]
struct PostEmbedding {
    link: String,
    embedding: Vec<f32>,
}

#[server(SearchPosts, "/api")]
pub async fn semantic_search(query:String) -> Result<Vec<Poast>, ServerFnError> {
    let openai = Client::new();

    let query_embedding = openai
        .embeddings()
        .create(CreateEmbeddingRequestArgs::default()
            .model("text-embedding-3-small")
            .input(EmbeddingInput::String(query))
            .build()?)
        .await?
        .data[0]
        .embedding
        .clone();

    let supabase = crate::supabase::get_client();
    let response = supabase
        .from("post embeddings")
        .select("*")
        .execute()
        .await?;

    let embeddings: Vec<PostEmbedding> = serde_json::from_str(&response.text().await?)?;

    let mut results: Vec<(String, f32)> = embeddings
        .into_iter()
        .map(|post| {
            let similarity = cosine_similarity(&query_embedding, &post.embedding);
            (post.link, similarity)
        })
        .collect();

    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let links: Vec<String> = results.iter()
        .take(10)
        .map(|(link, _)| link.clone())
        .collect();

    let posts_response = supabase
        .from("poasts")
        .select("*")
        .in_("link", &links)
        .execute()
        .await?;

    let mut posts: Vec<Poast> = serde_json::from_str(&posts_response.text().await?)?;

    posts.sort_by(|a, b| {
        let a_score = results.iter().find(|(l, _)| l == &a.link).unwrap().1;
        let b_score = results.iter().find(|(l, _)| l == &b.link).unwrap().1;
        b_score.partial_cmp(&a_score).unwrap()
    });

    Ok(posts)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot_product / (norm_a * norm_b)
}

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
    }

    // posts w/o embeddings
    let response = supabase
        .from("poasts")
        .select("link, title, summary, full_text")
        .not("link", "in", "(SELECT link FROM post_embeddings)")
        .execute()
        .await?;

    let posts: Vec<Poast> = serde_json::from_str(&response.text().await?)?;

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
