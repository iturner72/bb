use::leptos::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{EventSource, MessageEvent, ErrorEvent};
use std::collections::HashMap;

use crate::server_fn::RssProgressUpdate;

#[component]
pub fn BatchProcessor() -> impl IntoView {
    let (progress_states, set_progress_states) = create_signal::<HashMap<String, RssProgressUpdate>>(HashMap::new());

    let start_backfill = move || {
        let event_source = EventSource::new("/api/backfill-progress")
            .expect("Failed to connect to SSE endpoint");

        let event_source_clone = event_source.clone();

        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Some(data) = event.data().as_string() {
                if data == "[DONE]" {
                    event_source_clone.close();
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
        <div class="p-4">
            <button
                class="px-4 py-2 bg-seafoam-500 text-white rounded hover:bg-seafoam-400"
                on:click=move |_| start_backfill()
            >
                "Start Backfill"
            </button>
            
            <div class="mt-4">
                {move || {
                    let states = progress_states.get();
                    states.values().map(|update| {
                        view! {
                            <div class="mb-2 p-2 text-gray-500 dark:text-gray-600 border rounded">
                                <div>"Company: " {&update.company}</div>
                                <div>"Status: " {&update.status}</div>
                                <div>"Posts Processed: " {update.new_posts}</div>
                                {update.current_post.as_ref().map(|post| view! {
                                    <div>"Current: " {post}</div>
                                })}
                            </div>
                        }
                    }).collect_view()
                }}
            </div>
        </div>
    }
}

