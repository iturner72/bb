use axum::{
    response::sse::{Event, Sse},
    extract::{Query, State},
    Json
};
use std::collections::HashMap;
use std::convert::Infallible;
use tokio::sync::mpsc as tokio_mpsc;
use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{
    cancellable_sse::{create_cancellable_sse_stream, CancellableSseStream},
    state::AppState,
    types::StreamResponse,
    components::search::SearchType,
    local_llm_service::download_llm_models::ModelType,

};

pub struct SseStream {
    pub receiver: tokio_mpsc::Receiver<Result<Event, Infallible>>,
}

impl Stream for SseStream {
    type Item = Result<Event, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

pub async fn create_stream(
    State(state): State<AppState>,
) -> Json<StreamResponse> {
    let stream_id = uuid::Uuid::new_v4().to_string();
    state.sse_state.register_stream(stream_id.clone());
    Json(StreamResponse { stream_id })
}

pub async fn embeddings_generation_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Sse<CancellableSseStream> {
    let stream_id = params
        .get("stream_id")
        .cloned()
        .expect("stream_id is required");

    create_cancellable_sse_stream(
        state.sse_state,
        stream_id,
        |tx, token| async move {
            crate::embedding_service::embeddings::generate_embeddings(tx, token).await
    }).await
}

pub async fn local_embeddings_generation_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Sse<CancellableSseStream> {
    let stream_id = params
        .get("stream_id")
        .cloned()
        .expect("stream_id is required");

    create_cancellable_sse_stream(
        state.sse_state,
        stream_id,
        |tx, token| async move {
            crate::embeddings_service::embeddings_local::generate_local_embeddings(tx, token).await
    }).await
}

pub async fn rss_progress_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Sse<CancellableSseStream> {
    let stream_id = params
        .get("stream_id")
        .cloned()
        .expect("stream_id is required");

    create_cancellable_sse_stream(
        state.sse_state,
        stream_id,
        |tx, token| async move {
            crate::rss_service::server::process_feeds_with_progress(tx, token).await
        },
    ).await
}

pub async fn backfill_progress_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Sse<CancellableSseStream> {
    let stream_id = params
        .get("stream_id")
        .cloned()
        .expect("stream_id is required");

    create_cancellable_sse_stream(
        state.sse_state,
        stream_id,
        |tx, token| async move {
            crate::backfill_service::backfill::backfill_missing_data(tx, token).await
        },
    ).await
}

pub async fn refresh_summaries_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Sse<CancellableSseStream> {
    let stream_id = params
        .get("stream_id")
        .cloned()
        .expect("stream_id is required");

    let company = params.get("company").cloned();
    let start_year = params
        .get("start_year")
        .and_then(|y| y.parse::<i32>().ok());
    let end_year = params
        .get("end_year")
        .and_then(|y| y.parse::<i32>().ok());

    create_cancellable_sse_stream(
        state.sse_state,
        stream_id,
        move |tx, token| async move {
            crate::summary_refresh_service::refresh::refresh_summaries(
                tx,
                token,
                company,
                start_year,
                end_year
            ).await
        },
    ).await
}

pub async fn rag_query_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Sse<CancellableSseStream> {
    let stream_id = params
        .get("stream_id")
        .cloned()
        .expect("stream_id is required");

    let query = params
        .get("query")
        .cloned()
        .expect("query is required");

    let search_type = params
        .get("search_type")
        .map(|s| match s.as_str() {
            "openai" => SearchType::OpenAISemantic,
            "local" => SearchType::LocalSemantic,
            _ => SearchType::OpenAISemantic,
        })
        .unwrap_or(SearchType::OpenAISemantic);

    // Check if local LLM should be used
    let use_local_llm = params
        .get("llm_provider")
        .map(|s| s == "local")
        .unwrap_or(false);

    create_cancellable_sse_stream(
        state.sse_state,
        stream_id,
        move |tx, _token| async move {
            use crate::rag_service::rag::rag::create_rag_service;
            
            match create_rag_service(use_local_llm) {
                Ok(rag_service) => {
                    rag_service.process_query(query, search_type, tx).await
                }
                Err(e) => {
                    log::error!("Failed to create RAG service: {}", e);
                    Err(e)
                }
            }
        },
    ).await
}

// Add a new handler specifically for local LLM testing
pub async fn local_llm_test_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Sse<CancellableSseStream> {
    let stream_id = params
        .get("stream_id")
        .cloned()
        .expect("stream_id is required");

    let prompt = params
        .get("prompt")
        .cloned()
        .unwrap_or_else(|| "What is cryptocurrency?".to_string());

    create_cancellable_sse_stream(
        state.sse_state,
        stream_id,
        move |tx, _token| async move {
            use crate::local_llm_service::local_llm::local_llm::{LocalLLMService, GenerationConfig};
            use crate::rag_service::rag::rag::RagResponse;
            use axum::response::sse::Event;
            
            match LocalLLMService::init(ModelType::SmolLM2135M) {
                Ok(_) => {
                    if let Ok(service) = LocalLLMService::get_instance() {
                        let formatted_prompt = service.format_chat_prompt(
                            "You are a helpful assistant.",
                            &prompt,
                            "Test context for local LLM.",
                        );
                        
                        let config = GenerationConfig::default();
                        service.generate_streaming_response(formatted_prompt, tx, config).await
                            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)
                    } else {
                        // Send error response
                        let response = RagResponse {
                            message_type: "error".to_string(),
                            content: Some("Local LLM service not available".to_string()),
                            citations: None,
                        };
                        let json = serde_json::to_string(&response).unwrap();
                        let _ = tx.send(Ok(Event::default().data(json))).await;
                        Ok(())
                    }
                }
                Err(e) => {
                    log::error!("Failed to initialize local LLM: {}", e);
                    let response = RagResponse {
                        message_type: "error".to_string(),
                        content: Some(format!("Failed to initialize local LLM: {}", e)),
                        citations: None,
                    };
                    let json = serde_json::to_string(&response).unwrap();
                    let _ = tx.send(Ok(Event::default().data(json))).await;
                    Ok(())
                }
            }
        },
    ).await
}
