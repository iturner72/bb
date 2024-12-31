use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{EventSource, MessageEvent, ErrorEvent};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Datelike};
use serde::{Deserialize, Serialize};

use crate::server_fn::RssProgressUpdate;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompanyLink {
    pub company: String,
    pub link: String,
    pub last_processed: Option<DateTime<Utc>>,
}

#[server(GetCompanyLinks, "/api")]
pub async fn get_company_links() -> Result<Vec<CompanyLink>, ServerFnError> {
    use crate::supabase::get_client;

    let client = get_client();

    let response = client
        .from("links")
        .select("company, link, last_processed")
        .execute()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let text = response.text()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    serde_json::from_str(&text)
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[component]
pub fn SummaryRefreshProcessor() -> impl IntoView {
    let (progress_states, set_progress_states) = signal::<HashMap<String, RssProgressUpdate>>(HashMap::new());
    let (is_processing, set_is_processing) = signal(false);
    let (selected_company, set_selected_company) = signal::<Option<String>>(None);

    let current_year = chrono::Local::now().year();
    let (start_year, set_start_year) = signal(Option::<i32>::None);
    let (end_year, set_end_year) = signal(Option::<i32>::None);

    let year_options = (2020..=current_year).rev().collect::<Vec<i32>>();

    let companies = Resource::new(
        || (),
        |_| async move {
            get_company_links()
                .await
                .map(|links| {
                    let mut companies: Vec<String> = links
                        .into_iter()
                        .map(|link| link.company)
                        .collect();
                    companies.sort();
                    companies
                })
        }
    );

    let start_refresh = move || {
        set_is_processing(true);
        set_progress_states.update(|states| states.clear());

        let company = selected_company.get();

        let mut url_parts = Vec::new();

        if let Some(c) = company {
            url_parts.push(format!("company={}", urlencoding::encode(&c)));
        }

        if let Some(start) = start_year.get() {
            url_parts.push(format!("start_year={}", start));
        }

        if let Some(end) = end_year.get() {
            url_parts.push(format!("end_year={}", end));
        }

        let url = if url_parts.is_empty() {
            "api/refresh-summaries".to_string()
        } else {
            format!("/api/refresh-summaries?{}", url_parts.join("&"))
        };

        let event_source = EventSource::new(&url)
            .expect("Failed to connect to SSE endpoint");

        let event_source_clone = event_source.clone();

        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Some(data) = event.data().as_string() {
                if data == "[DONE]" {
                    event_source_clone.close();
                    set_is_processing(false);
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
            set_is_processing(false);
        }) as Box<dyn FnMut(_)>);

        event_source.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        event_source.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        on_message.forget();
        on_error.forget();
    };

    view! {
        <div class="p-4 space-y-4">
            <div class="flex items-center space-x-4">
                <div class="w-64">
                    <select
                        class="w-full p-2 rounded-md bg-gray-100 dark:bg-teal-800 
                               text-gray-800 dark:text-gray-200 
                               border border-teal-500 dark:border-seafoam-500
                               focus:border-seafoam-600 dark:focus:border-aqua-400
                               focus:outline-none focus:ring-2 focus:ring-seafoam-500 dark:focus:ring-aqua-400"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            set_selected_company(if value.is_empty() { None } else { Some(value) });
                        }
                    >
                        <option value="">"All Companies"</option>
                        <Suspense>
                            {move || {
                                companies.get().map(|result| {
                                    match result {
                                        Ok(companies) => companies.into_iter().map(|company| {
                                            view! {
                                                <option value={company.clone()}>{company.clone()}</option>
                                            }.into_any()
                                        }).collect_view(),
                                        Err(_) => vec![
                                            view! { 
                                                <option>"Error loading companies"</option> 
                                            }.into_any()
                                        ].collect_view(),
                                    }
                                })
                            }}
                        </Suspense>
                    </select>
                </div>

                // Year Range Selection
                <div class="w-full sm:w-40">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "Start Year"
                    </label>
                    <select
                        class="w-full p-2 rounded-md bg-gray-100 dark:bg-teal-800 
                               text-gray-800 dark:text-gray-200 
                               border border-teal-500 dark:border-seafoam-500
                               focus:border-seafoam-600 dark:focus:border-aqua-400
                               focus:outline-none focus:ring-2 focus:ring-seafoam-500 dark:focus:ring-aqua-400"
                        prop:value=move || start_year.get().map(|y| y.to_string()).unwrap_or_default()
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            if value.is_empty() {
                                set_start_year(None);
                            } else if let Ok(year) = value.parse::<i32>() {
                                set_start_year(Some(year));
                                if let Some(end_y) = end_year.get() {
                                    if year > end_y {
                                        set_end_year(Some(year));
                                    }
                                }
                            }
                        }
                    >
                        {year_options.clone().into_iter().map(|year| {
                            view! {
                                <option value={year.to_string()}>{year.to_string()}</option>
                            }
                        }).collect_view()}
                    </select>
                </div>

                <div class="w-full sm:w-40">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "End Year"
                    </label>
                    <select
                        class="w-full p-2 rounded-md bg-gray-100 dark:bg-teal-800 
                               text-gray-800 dark:text-gray-200 
                               border border-teal-500 dark:border-seafoam-500
                               focus:border-seafoam-600 dark:focus:border-aqua-400
                               focus:outline-none focus:ring-2 focus:ring-seafoam-500 dark:focus:ring-aqua-400"
                        prop:value=move || end_year.get().map(|y| y.to_string()).unwrap_or_default()
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            if value.is_empty() {
                                set_end_year(None);
                            } else if let Ok(year) = value.parse::<i32>() {
                                set_end_year(Some(year));
                                if let Some(start_y) = start_year.get() {
                                    if year < start_y {
                                        set_start_year(Some(year));
                                    }
                                }
                            }
                        }
                    >
                        {year_options.into_iter().map(|year| {
                            view! {
                                <option value={year.to_string()}>{year.to_string()}</option>
                            }
                        }).collect_view()}
                    </select>
                </div>

                <button
                    class="px-4 py-2 bg-seafoam-500 dark:bg-seafoam-600 text-white rounded 
                           hover:bg-seafoam-400 dark:hover:bg-seafoam-500 transition-colors
                           disabled:bg-gray-400 dark:disabled:bg-gray-600 disabled:cursor-not-allowed"
                    on:click=move |_| start_refresh()
                    disabled=is_processing
                >
                    {move || if is_processing() { "Processing..." } else { "Refresh Summaries" }}
                </button>
                
                {move || is_processing().then(|| view! {
                    <span class="text-sm text-seafoam-600 dark:text-seafoam-400">
                        "Refreshing summaries..."
                    </span>
                })}
            </div>

            {move || {
                let states = progress_states.get();
                if !states.is_empty() {
                    view! {
                        <div class="grid gap-3">
                            {states.values().map(|update| {
                                let is_completed = update.status == "completed";
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
                                                {update.company.clone()}
                                            </span>
                                            <span class=format!("px-2 py-1 text-sm rounded {}", status_class)>
                                                {update.status.clone()}
                                            </span>
                                        </div>
                                        
                                        <div class="space-y-2 text-sm">
                                            <div class="grid grid-cols-2 text-gray-600 dark:text-gray-300">
                                                <span>"Processed"</span>
                                                <span class="text-right">{update.new_posts}</span>
                                            </div>
                                            <div class="grid grid-cols-2 text-gray-600 dark:text-gray-300">
                                                <span>"Failed"</span>
                                                <span class="text-right">{update.skipped_posts}</span>
                                            </div>
                                            {update.current_post.as_ref().map(|post| {
                                                let post = post.clone();
                                                view! {
                                                    <div class="mt-2">
                                                        <span class="text-gray-500 dark:text-gray-400">"Current: "</span>
                                                        <span class="text-gray-700 dark:text-gray-200 line-clamp-1">
                                                            {move || post.clone()}
                                                        </span>
                                                    </div>
                                                }
                                            })}
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
