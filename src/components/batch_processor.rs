use::leptos::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{EventSource, MessageEvent, ErrorEvent};
use std::collections::HashMap;

use crate::server_fn::RssProgressUpdate;

#[component]
pub fn BatchProcessor() -> impl IntoView {
    let (progress_states, set_progress_states) = create_signal::<HashMap<String, RssProgressUpdate>>(HashMap::new());
    let (is_processing, set_is_processing) = create_signal(false);
    let (no_posts_to_process, set_no_posts_to_process) = create_signal(false);

    let start_backfill = move || {
        set_is_processing(true);
        set_no_posts_to_process(false);
        set_progress_states.update(|states| states.clear());

        let event_source = EventSource::new("/api/backfill-progress")
            .expect("Failed to connect to SSE endpoint");

        let event_source_clone = event_source.clone();

        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Some(data) = event.data().as_string() {
                if data == "[DONE]" {
                    event_source_clone.close();
                    set_is_processing(false);

                    if progress_states.get().is_empty() {
                        set_no_posts_to_process(true);
                    }
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

        on_message.forget();
        on_error.forget();
    };

    view! {
        <div class="p-4 space-y-4">
            <div class="flex items-center justify-between">
                <button
                    class="px-4 py-2 bg-seafoam-500 dark:bg-seafoam-600 text-white rounded 
                           hover:bg-seafoam-400 dark:hover:bg-seafoam-500 transition-colors
                           disabled:bg-gray-400 dark:disabled:bg-gray-600 disabled:cursor-not-allowed"
                    on:click=move |_| start_backfill()
                    disabled=is_processing
                >
                    {move || if is_processing() { "Processing..." } else { "Start Backfill" }}
                </button>
                
                {move || is_processing().then(|| view! {
                    <span class="text-sm text-seafoam-600 dark:text-seafoam-400">
                        "Processing posts..."
                    </span>
                })}
            </div>

            {move || no_posts_to_process().then(|| view! {
                <div class="p-4 bg-gray-100 dark:bg-teal-800 rounded-lg border-l-4 border-mint-500 dark:border-mint-400">
                    <p class="text-gray-700 dark:text-gray-200">
                        "No posts found that need backfilling! All posts have their summaries and buzzwords."
                    </p>
                </div>
            })}

            {move || {
                let states = progress_states.get();
                if !states.is_empty() {
                    view! {
                        <div class="grid gap-3">
                            {states.values().map(|update| {
                                let is_completed = update.status == "completed" || update.status == "backfilled";
                                let status_class = if is_completed {
                                    "bg-seafoam-100 dark:bg-seafoam-900 text-seafoam-800 dark:text-seafoam-200"
                                } else {
                                    "bg-aqua-100 dark:bg-aqua-900 text-aqua-800 dark:text-aqua-200"
                                };
                                let border_class = if is_completed {
                                    "border-seafoam-500 dark:border-mint-400"
                                } else {
                                    "border-aqua-500 dark:border-aqua-400"
                                };
                                
                                view! {
                                    <div class=format!("p-4 rounded-lg border-l-4 bg-gray-100 dark:bg-teal-800 {}", border_class)>
                                        <div class="flex justify-between items-center mb-2">
                                            <span class="font-medium text-gray-800 dark:text-gray-200">
                                                {&update.company}
                                            </span>
                                            <span class=format!("px-2 py-1 text-sm rounded {}", status_class)>
                                                {&update.status}
                                            </span>
                                        </div>
                                        
                                        <div class="space-y-2 text-sm">
                                            <div class="grid grid-cols-2 text-gray-600 dark:text-gray-300">
                                                <span>"Processed"</span>
                                                <span class="text-right">{update.new_posts}</span>
                                            </div>
                                            <div class="grid grid-cols-2 text-gray-600 dark:text-gray-300">
                                                <span>"Skipped"</span>
                                                <span class="text-right">{update.skipped_posts}</span>
                                            </div>
                                            {update.current_post.as_ref().map(|post| view! {
                                                <div class="mt-2">
                                                    <span class="text-gray-500 dark:text-gray-400">"Current: "</span>
                                                    <span class="text-gray-700 dark:text-gray-200 line-clamp-1">
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
                } else {
                    view! { <div></div> }
                }
            }}
        </div>
    }
}
