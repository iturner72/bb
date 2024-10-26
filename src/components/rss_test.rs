use leptos::*;
use leptos_router::ActionForm;
use crate::server_fn::{TriggerRssFetchStream, RssProgressUpdate};
use std::collections::HashMap;

#[component]
pub fn RssTest() -> impl IntoView {
    let trigger_action = create_server_action::<TriggerRssFetchStream>();
    
    // Store the latest state for each company
    let (company_states, set_company_states) = create_signal::<HashMap<String, RssProgressUpdate>>(HashMap::new());
    
    // Track completed companies
    let completed_companies = create_memo(move |_| {
        company_states.get()
            .iter()
            .filter(|(_, status)| status.status == "completed")
            .map(|(company, _)| company.to_string())
            .collect::<Vec<_>>()
    });

    // Create an effect to update company states as updates come in
    create_effect(move |_| {
        if let Some(Ok(updates)) = trigger_action.value().get() {
            let mut new_states = HashMap::new();
            for update in updates {
                new_states.insert(update.company.clone(), update);
            }
            set_company_states.set(new_states);
        }
    });

    // Helper function to determine card classes based on status
    let get_card_classes = move |update: &RssProgressUpdate| {
        let base_classes = "transition-all duration-300 transform bg-gray-800 p-3 rounded-lg";
        let status_classes = match update.status.as_str() {
            "completed" => "border-l-4 border-green-500 translate-x-1",
            "processing" => "border-l-4 border-yellow-500",
            _ => "border-l-4 border-gray-500 opacity-50"
        };
        format!("{} {}", base_classes, status_classes)
    };

    // Helper function to get status badge classes
    let get_status_badge_classes = move |status: &str| {
        let base_classes = "text-sm px-2 py-1 rounded";
        let status_classes = if status == "completed" {
            "bg-green-900/30 text-green-400"
        } else {
            "bg-yellow-900/30 text-yellow-400"
        };
        format!("{} {}", base_classes, status_classes)
    };

    view! {
        <div class="p-4 space-y-4">
            <ActionForm action=trigger_action>
                <button
                    type="submit"
                    class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:bg-gray-400 transition-colors"
                    disabled=move || trigger_action.pending().get()
                >
                    {move || if trigger_action.pending().get() {
                        view! {
                            <>
                            <span class="inline-flex items-center">
                                <span class="animate-spin mr-2">"↻"</span>
                                "Processing Feeds..."
                            </span>
                            </>
                        }
                    } else {
                        view! { <>"Trigger RSS Fetch"</> }
                    }}
                </button>
            </ActionForm>

            {move || {
                let states = company_states.get();
                if states.is_empty() {
                    view! {
                        <div class="mt-4 p-4 text-center animate-pulse text-gray-400">
                            "Waiting to start processing..."
                        </div>
                    }
                } else {
                    view! {
                        <div class="mt-4 space-y-2">
                            <h3 class="text-lg font-semibold text-gray-200 mb-4">
                                "Feed Processing Results "
                                <span class="text-sm text-gray-400">
                                    {format!("({} completed)", completed_companies.get().len())}
                                </span>
                            </h3>
                            <div class="grid gap-3">
                                {states.values().map(|update| {
                                    let status = update.status.clone();
                                    let company = update.company.clone();
                                    let new_posts = update.new_posts;
                                    let skipped_posts = update.skipped_posts;
                                    let current_post = update.current_post.clone();
                                    
                                    view! {
                                        <div class={get_card_classes(update)}>
                                            <div class="flex justify-between items-center">
                                                <span class="text-purple-300 font-medium">
                                                    {company}
                                                </span>
                                                <span class={get_status_badge_classes(&status)}>
                                                    {status}
                                                </span>
                                            </div>
                                            <div class="mt-2 text-gray-300 text-sm grid gap-1">
                                                <div class="flex justify-between">
                                                    <span>"New posts"</span>
                                                    <span class="text-teal-400">{new_posts}</span>
                                                </div>
                                                <div class="flex justify-between">
                                                    <span>"Skipped posts"</span>
                                                    <span class="text-gray-500">{skipped_posts}</span>
                                                </div>
                                                {current_post.map(|post| view! {
                                                    <div class="mt-1 text-sm">
                                                        <span class="text-gray-500">"Processing: "</span>
                                                        <span class="text-teal-400 truncate block">{post}</span>
                                                    </div>
                                                })}
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    }
                }
            }}
        </div>
    }
}
