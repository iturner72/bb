use axum::{
    extract::Query,
    response::sse::{Event, Sse},
};
use std::collections::HashMap;
use std::convert::Infallible;
use tokio::sync::mpsc as tokio_mpsc;
use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct SseStream {
    pub receiver: tokio_mpsc::Receiver<Result<Event, Infallible>>,
}

impl Stream for SseStream {
    type Item = Result<Event, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

pub async fn rss_progress_handler(
    Query(_params): Query<HashMap<String, String>>,
) -> Sse<SseStream> {
    let (tx, rx) = tokio_mpsc::channel(100);

    tokio::spawn(async move {
        if let Err(e) = crate::rss_service::server::process_feeds_with_progress(tx).await {
            log::error!("Error processing feeds: {}", e);
        }
    });

    Sse::new(SseStream { receiver: rx })
}

pub async fn backfill_progress_handler(
    Query(_params): Query<HashMap<String, String>>,
) -> Sse<SseStream> {
    let (tx, rx) = tokio_mpsc::channel(100);

    tokio::spawn(async move {
        if let Err(e) = crate::backfill_service::backfill::backfill_missing_data(tx).await {
            log::error!("Error during backfill: {}", e);
        }
    });

    Sse::new(SseStream { receiver: rx })
}

pub async fn refresh_summaries_handler(
    Query(params): Query<HashMap<String, String>>,
) -> Sse<SseStream> {
    let (tx, rx) = tokio_mpsc::channel(100);

    let company = params.get("company").cloned();

    tokio::spawn(async move {
        if let Err(e) = crate::summary_refresh_service::refresh::refresh_summaries(tx, company).await {
            log::error!("Error refreshing summaries: {}", e);
        }
    });

    Sse::new(SseStream { receiver: rx})
}

