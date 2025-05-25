pub mod app;
pub mod auth;
pub mod backfill_service;
#[cfg(feature = "ssr")]
pub mod cancellable_sse;
pub mod components;
pub mod embedding_service;
pub mod embeddings_service;
pub mod error_template;
pub mod handlers;
pub mod rag_service;
pub mod rss_service;
pub mod server_fn;
pub mod state;
pub mod summary_refresh_service;
pub mod supabase;
pub mod types;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
