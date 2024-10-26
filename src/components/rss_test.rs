use leptos::*;
use leptos_router::ActionForm;
use crate::server_fn::{TriggerRssFetchStream, RssProgressUpdate};
use std::collections::HashMap;

#[component]
pub fn RssTest() -> impl IntoView {
    let trigger_action = create_server_action::<TriggerRssFetchStream>();
    
    let (company_states, set_company_states) = create_signal::<HashMap<String, RssProgressUpdate>>(HashMap::new());
    
    let completed_companies = create_memo(move |_| {
        company_states.get()
            .iter()
            .filter(|(_, status)| status.status == "completed")
            .map(|(company, _)| company.to_string())
            .collect::<Vec<_>>()
    });

    create_effect(move |_| {
        if let Some(Ok(updates)) = trigger_action.value().get() {
            let mut new_states = HashMap::new();
            for update in updates {
                new_states.insert(update.company.clone(), update);
            }
            set_company_states.set(new_states);
        }
    });

    let get_card_classes = move |update: &RssProgressUpdate| {
        let base_classes = "transition-all duration-300 transform bg-gray-200 dark:bg-teal-900 p-3 rounded-lg";
        let status_classes = match update.status.as_str() {
            "completed" => "border-l-4 border-seafoam-500 dark:border-mint-800 translate-x-1",
            "processing" => "border-l-4 border-aqua-500 dark:border-seafoam-400",
            _ => "border-l-4 border-gray-400 dark:border-gray-800 opacity-50"
        };
        format!("{} {}", base_classes, status_classes)
    };

    let get_status_badge_classes = move |status: &str| {
        let base_classes = "text-sm px-2 py-1 rounded";
        let status_classes = if status == "completed" {
            "bg-seafoam-200 dark:bg-mint-900/30 text-seafoam-900 dark:text-mint-700"
        } else {
            "bg-aqua-200 dark:bg-seafoam-900/30 text-aqua-900 dark:text-seafoam-300"
        };
        format!("{} {}", base_classes, status_classes)
    };

    view! {
        <div class="p-4 space-y-4">
            <ActionForm action=trigger_action>
                <button
                    type="submit"
                    class="px-4 py-2 bg-seafoam-500 dark:bg-aqua-600 text-white dark:text-teal-100 rounded 
                           hover:bg-seafoam-400 dark:hover:bg-aqua-500 
                           disabled:bg-gray-300 dark:disabled:bg-gray-800 
                           disabled:text-gray-500 dark:disabled:text-gray-600 
                           transition-colors"
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
                        <div class="mt-4 p-4 text-center animate-pulse text-gray-500 dark:text-gray-600">
                            "Waiting to start processing..."
                        </div>
                    }
                } else {
                    view! {
                        <div class="mt-4 space-y-2">
                            <h3 class="text-lg font-semibold text-gray-800 dark:text-gray-400 mb-4">
                                "Feed Processing Results "
                                <span class="text-sm text-gray-500 dark:text-gray-600">
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
                                                <span class="text-seafoam-800 dark:text-mint-600 font-medium">
                                                    {company}
                                                </span>
                                                <span class={get_status_badge_classes(&status)}>
                                                    {status}
                                                </span>
                                            </div>
                                            <div class="mt-2 text-gray-700 dark:text-gray-500 text-sm grid gap-1">
                                                <div class="flex justify-between">
                                                    <span>"New posts"</span>
                                                    <span class="text-aqua-600 dark:text-aqua-300">{new_posts}</span>
                                                </div>
                                                <div class="flex justify-between">
                                                    <span>"Skipped posts"</span>
                                                    <span class="text-gray-500 dark:text-gray-600">{skipped_posts}</span>
                                                </div>
                                                {current_post.map(|post| view! {
                                                    <div class="mt-1 text-sm">
                                                        <span class="text-gray-500 dark:text-gray-600">"Processing: "</span>
                                                        <span class="text-seafoam-600 dark:text-seafoam-300 truncate block">
                                                            {post}
                                                        </span>
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
