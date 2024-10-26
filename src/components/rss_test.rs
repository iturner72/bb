use leptos::*;
use leptos_router::ActionForm;
use crate::server_fn::{TriggerRssFetch, RssFetchProgress};

#[component]
pub fn RssTest() -> impl IntoView {
    let trigger_action = create_server_action::<TriggerRssFetch>();
    
    view! {
        <div class="p-4 space-y-4">
            <ActionForm action=trigger_action>
                <button
                    type="submit"
                    class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:bg-gray-400"
                    disabled=move || trigger_action.pending().get()
                >
                    {move || if trigger_action.pending().get() { "Fetching..." } else { "Trigger RSS Fetch" }}
                </button>
            </ActionForm>
            {move || {
                trigger_action.value().get().map(|result| {
                    match result {
                        Ok(progress) => view! {
                            <div class="mt-4 space-y-2">
                                <h3 class="text-lg font-semibold text-gray-200">Feed Processing Results</h3>
                                <div class="grid gap-3">
                                    {progress.into_iter().map(|p| view! {
                                        <div class="bg-gray-800 p-3 rounded-lg">
                                            <div class="flex justify-between items-center">
                                                <span class="text-purple-300 font-medium">{p.company}</span>
                                                <span class="text-teal-400 text-sm">{"Status: "} {p.status}</span>
                                            </div>
                                            <div class="mt-2 text-gray-300 text-sm">
                                                <div>{"New posts: "} {p.new_posts}</div>
                                                <div>{"Skipped posts: "} {p.skipped_posts}</div>
                                            </div>
                                        </div>
                                    }).collect_view()}
                                </div>
                            </div>
                        },
                        Err(e) => view! {
                            <div class="p-4 rounded bg-red-100 text-red-800">"Error: " {e.to_string()}</div>
                        }
                    }
                })
            }}
        </div>
    }
}
