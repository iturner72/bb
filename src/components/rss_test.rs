use leptos::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{EventSource, MessageEvent, ErrorEvent};
use std::collections::HashMap;
use crate::server_fn::RssProgressUpdate;

#[component]
pub fn RssTest() -> impl IntoView {
    let (progress_states, set_progress_states) = create_signal::<HashMap<String, RssProgressUpdate>>(HashMap::new());

    let start_streaming = move || {
        let event_source = EventSource::new("/api/rss-progress")
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
            event_source_error.close();
        }) as Box<dyn FnMut(_)>);

        event_source.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        event_source.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        
        // Keep the closures alive
        on_message.forget();
        on_error.forget();
    };

    view! {
        <div class="p-4 space-y-4">
            <button
                class="px-4 py-2 bg-seafoam-500 dark:bg-aqua-600 text-white dark:text-teal-100 rounded 
                       hover:bg-seafoam-400 dark:hover:bg-aqua-500 transition-colors"
                on:click=move |_| start_streaming()
            >
                "Start RSS Fetch"
            </button>

            {move || {
                let states = progress_states.get();
                if states.is_empty() {
                    view! {
                        <div class="mt-4 p-4 text-center text-gray-500 dark:text-gray-600">
                            "Waiting to start processing..."
                        </div>
                    }
                } else {
                    view! {
                        <div class="mt-4 grid gap-3">
                            {states.values().map(|update| {
                                let class = match update.status.as_str() {
                                    "completed" => "bg-gray-200 dark:bg-teal-900 p-3 rounded-lg border-l-4 border-seafoam-500 dark:border-mint-800 translate-x-1",
                                    "processing" => "bg-gray-200 dark:bg-teal-900 p-3 rounded-lg border-l-4 border-aqua-500 dark:border-seafoam-400",
                                    _ => "bg-gray-200 dark:bg-teal-900 p-3 rounded-lg border-l-4 border-gray-400 dark:border-gray-800 opacity-50"
                                };
                                let status_class = if update.status == "completed" {
                                    "text-sm px-2 py-1 rounded bg-seafoam-200 dark:bg-mint-900/30 text-seafoam-900 dark:text-mint-700"
                                } else {
                                    "text-sm px-2 py-1 rounded bg-aqua-200 dark:bg-seafoam-900/30 text-aqua-900 dark:text-seafoam-300"
                                };
                                view! {
                                    <div class={class}>
                                        <div class="flex justify-between items-center">
                                            <span class="text-seafoam-800 dark:text-mint-600 font-medium">
                                                {update.company.clone()}
                                            </span>
                                            <span class={status_class}>
                                                {update.status.clone()}
                                            </span>
                                        </div>
                                        <div class="mt-2 text-sm grid gap-1">
                                            <div class="flex justify-between">
                                                <span>"New posts"</span>
                                                <span>{update.new_posts}</span>
                                            </div>
                                            <div class="flex justify-between">
                                                <span>"Skipped posts"</span>
                                                <span>{update.skipped_posts}</span>
                                            </div>
                                            {update.current_post.as_ref().map(|post| view! {
                                                <div class="mt-1">
                                                    <span class="text-gray-500">"Processing: "</span>
                                                    <span class="text-seafoam-600 dark:text-seafoam-300 truncate">
                                                        {post}
                                                    </span>
                                                </div>
                                            })}
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }
                }
            }}
        </div>
    }
}
