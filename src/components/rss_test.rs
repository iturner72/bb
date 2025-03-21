use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{EventSource, MessageEvent, ErrorEvent};
use std::collections::HashMap;

use crate::{
    types::StreamResponse,
    server_fn::RssProgressUpdate
};

#[component]
pub fn RssTest() -> impl IntoView {
    let (progress_states, set_progress_states) = signal::<HashMap<String, RssProgressUpdate>>(HashMap::new());
    let (is_processing, set_is_processing) = signal(false);
    let (current_stream_id, set_current_stream_id) = signal(Option::<String>::None);

    let cancel_stream = move || {
        if let Some(stream_id) = current_stream_id.get() {
            // use fetch API through web_sys
            let window = web_sys::window().unwrap();
            let url = format!("/api/cancel-stream?stream_id={}", stream_id);

            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(_resp) = JsFuture::from(window.fetch_with_str(&url)).await {
                    log::info!("Stream cancelled");
                }
            });
            set_is_processing(false);
            set_current_stream_id(None);
        }
    };

    let start_streaming = move || {
        set_is_processing(true);
        set_progress_states.update(|states| states.clear());

        let window = web_sys::window().unwrap();

        wasm_bindgen_futures::spawn_local(async move {
            let resp_value = match JsFuture::from(window.fetch_with_str("/api/create-stream")).await {
                Ok(val) => val,
                Err(e) => {
                    log::error!("Failed to fetch: {:?}", e);
                    set_is_processing(false);
                    return;
                }
            };

            let resp = resp_value.dyn_into::<web_sys::Response>().unwrap();
            let json = match JsFuture::from(resp.json().unwrap()).await {
                Ok(json) => json,
                Err(e) => {
                    log::error!("Failed to parse JSON: {:?}", e);
                    set_is_processing(false);
                    return;
                }
            };

            let stream_data: StreamResponse = serde_wasm_bindgen::from_value(json).unwrap();
            let stream_id = stream_data.stream_id;

            set_current_stream_id(Some(stream_id.clone()));

            let url = format!("/api/rss-progress?stream_id={}", stream_id);
            let event_source = EventSource::new(&url)
                .expect("Failed to connect to SSE endpoint");

            let event_source_clone = event_source.clone();

            let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
                if let Some(data) = event.data().as_string() {
                    if data == "[DONE]" { event_source_clone.close();
                    } else {
                        match serde_json::from_str::<RssProgressUpdate>(&data) {
                            Ok(update) => {
                                set_progress_states.update(|states| {
                                    states.insert(update.company.clone(), update);
                                });
                            },
                            Err(e) => log::error!("Failed to parse update: {}", e)
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);

            let event_source_error = event_source.clone();

            let on_error = Closure::wrap(Box::new(move |error: ErrorEvent| {
                log::error!("SSE Error: {:?}", error);
                if let Some(es) = error.target()
                    .and_then(|t| t.dyn_into::<web_sys::EventSource>().ok()) 
                {
                    if es.ready_state() == web_sys::EventSource::CLOSED {
                        if let Some(window) = web_sys::window() {
                            let _ = window.location().set_href("/admin");
                        }
                    }
                }
                event_source_error.close();
            }) as Box<dyn FnMut(_)>);

            event_source.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            event_source.set_onerror(Some(on_error.as_ref().unchecked_ref()));

            // Keep the closures alive
            on_message.forget();
            on_error.forget();
        });
    };

    view! {
        <div class="p-4 space-y-4 max-w-3xl mx-auto">
            <div class="flex items-center justify-between">
                <button
                    class="px-4 py-2 bg-seafoam-500 dark:bg-aqua-600 text-white dark:text-teal-100 rounded 
                           hover:bg-seafoam-400 dark:hover:bg-aqua-500 transition-colors
                           disabled:bg-gray-400 dark:disabled:bg-gray-600 disabled:cursor-not-allowed"
                    on:click=move |_| if is_processing.get() { cancel_stream() } else { start_streaming() }
                >
                    {move || if is_processing() { "Cancel" } else { "Start RSS Fetch" }}
                </button>
                <div class="text-sm text-seafoam-400 dark:text-seafoam-600">
                    {move || {
                        let states = progress_states.get();
                        let completed = states.values().filter(|s| s.status == "completed").count();
                        if !states.is_empty() {
                            format!("{}/{} completed", completed, states.len())
                        } else {
                            "".to_string()
                        }
                    }}
                </div>
            </div>

            {move || {
                let states = progress_states.get();
                if states.is_empty() {
                    view! {
                        <div class="mt-4 p-4 text-center text-gray-500 dark:text-gray-600">
                            "Waiting to start processing..."
                        </div>
                    }.into_any()
                } else {
                    let updates: Vec<_> = states.values().cloned().collect();
                    view! {
                        <div class="mt-4 grid gap-3">
                            <For
                                each=move || updates.clone()
                                key=|update| update.company.clone()
                                children=move |update| {
                                    let class = match update.status.as_str() {
                                        "completed" => "bg-gray-200 dark:bg-teal-900 p-3 rounded-lg border-l-4 border-seafoam-500 dark:border-mint-800 translate-x-1",
                                        "processing" => "bg-gray-200 dark:bg-teal-900 p-3 rounded-lg border-l-4 border-aqua-500 dark:border-seafoam-400",
                                        _ => "bg-gray-200 dark:bg-teal-900 p-3 rounded-lg border-l-4 border-gray-400 dark:border-gray-800 opacity-50"
                                    };
                                    let status_class = if update.status == "completed" {
                                        "text-xs md:text-sm px-2 py-1 rounded bg-seafoam-200 dark:bg-mint-900/30 text-seafoam-900 dark:text-mint-700"
                                    } else {
                                        "text-xs md:text-sm px-2 py-1 rounded bg-aqua-200 dark:bg-seafoam-900/30 text-aqua-900 dark:text-seafoam-300"
                                    };
                                    view! {
                                        <div class={class}>
                                            <div class="flex justify-between items-center gap-2">
                                                <span class="text-seafoam-800 dark:text-mint-600 font-medium textsm md:text-base truncate">
                                                    {move || update.company.clone()}
                                                </span>
                                                <span class={format!("{} whitespace-nowrap", status_class)}>
                                                    {move || update.status.clone()}
                                                </span>
                                            </div>
                                            <div class="mt-2 text-xs md:text-sm grid gap-1">
                                                <div class="grid grid-cols-2 text-seafoam-800 dark:text-mint-600">
                                                    <span>"New posts"</span>
                                                    <span class="text-right">{move || update.new_posts}</span>
                                                </div>
                                                <div class="grid grid-cols-2 text-seafoam-800 dark:text-mint-600">
                                                    <span>"Skipped posts"</span>
                                                    <span class="text-right">{move || update.skipped_posts}</span>
                                                </div>
                                                {update.current_post.as_ref().map(|post| {
                                                    let post = post.clone();
                                                    view! {
                                                        <div class="mt-2 space-y-1">
                                                            <span class="text-gray-500 text-xs">"Processing: "</span>
                                                            <span class="text-seafoam-600 dark:text-seafoam-300 text-sm line-clamp-2">
                                                                {move || post.clone()}
                                                            </span>
                                                        </div>
                                                    }
                                                })}
                                            </div>
                                        </div>
                                    }
                                }
                            />
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
