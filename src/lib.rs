pub mod app;
pub mod error_template;
pub mod state;
pub mod supabase;
pub mod components;
pub mod rss_service;
pub mod backfill_service;
pub mod summary_refresh_service;
pub mod auth;
pub mod server_fn;
pub mod handlers;
pub mod types;
#[cfg(feature = "ssr")]
pub mod cancellable_sse;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
