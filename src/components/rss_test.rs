use leptos::*;
use leptos_router::ActionForm;
use crate::server_fn::TriggerRssFetch;

#[component]
pub fn RssTest() -> impl IntoView {
    let trigger_action = create_server_action::<TriggerRssFetch>();

    view! {
        <div class="p-4 space-y-4">
            <ActionForm action=trigger_action>
                <button
                    type="submit"
                    class="px-4 py-2 bg-teal-500 text-mint-700 rounded hover:bg-teal-700 disabled:bg-wenge-400"
                    disabled=move || trigger_action.pending()
                >
                    {move || if trigger_action.pending().get() { "Fetching..." } else { "Trigger RSS Fetch" }}
                </button>
            </ActionForm>
            {move || trigger_action.value().get().map(|result| {
                match result {
                    Ok(message) => view! {
                        <div class="p-4 rounded bg-teal-400">{message}</div>
                    },
                    Err(e) => view! {
                        <div class="p-4 rounded bg-salmon-400">"Error: " {e.to_string()}</div>
                    }
                }
            })}
        </div>
    }
}
