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
    rag_service::rag::rag::RagService,
    components::search::SearchType,

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


// And add this handler function to src/handlers/sse.rs:
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

    create_cancellable_sse_stream(
        state.sse_state,
        stream_id,
        move |tx, _token| async move {
            let rag_service = RagService::new();
            rag_service.process_query(query, search_type, tx).await
        },
    ).await
}
