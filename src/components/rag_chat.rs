use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{ErrorEvent, EventSource, MessageEvent};

use crate::components::search::SearchType;
use crate::components::markdown::MarkdownRenderer;
use crate::types::StreamResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagMessage {
    pub role: String,
    pub content: String,
    pub citations: Option<Vec<Citation>>,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Citation {
    pub title: String,
    pub company: String,
    pub link: String,
    pub published_at: String,
    pub relevance_score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagResponse {
    pub message_type: String,
    pub content: Option<String>,
    pub citations: Option<Vec<Citation>>,
}

#[component]
pub fn RagChat() -> impl IntoView {
    let (messages, set_messages) = signal::<Vec<RagMessage>>(Vec::new());
    let (current_input, set_current_input) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let (current_response, set_current_response) = signal(String::new());
    let (current_citations, set_current_citations) = signal::<Vec<Citation>>(Vec::new());
    let (search_type, set_search_type) = signal(SearchType::OpenAISemantic);
    let (current_stream_id, set_current_stream_id) = signal(Option::<String>::None);
    let (status_message, set_status_message) = signal(String::new());

    let cancel_current_request = move || {
        if let Some(stream_id) = current_stream_id.get() {
            let window = web_sys::window().unwrap();
            let url = format!("/api/cancel-stream?stream_id={}", stream_id);

            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(_) = JsFuture::from(window.fetch_with_str(&url)).await {
                    log::info!("RAG stream cancelled");
                }
            });
            set_is_loading(false);
            set_current_stream_id(None);
            set_status_message(String::new());
        }
    };

    let send_message = move || {
        let query = current_input.get().trim().to_string();
        if query.is_empty() || is_loading.get() {
            return;
        }

        set_is_loading(true);
        set_current_response(String::new());
        set_current_citations(Vec::new());
        set_status_message(String::new());

        // Add user message to chat
        let user_message = RagMessage {
            role: "user".to_string(),
            content: query.clone(),
            citations: None,
            timestamp: js_sys::Date::new_0().to_iso_string().as_string().unwrap(),
        };

        set_messages.update(|msgs| msgs.push(user_message));
        set_current_input(String::new());

        let window = web_sys::window().unwrap();

        wasm_bindgen_futures::spawn_local(async move {
            // Create stream
            let resp_value = match JsFuture::from(window.fetch_with_str("/api/create-stream")).await {
                Ok(val) => val,
                Err(e) => {
                    log::error!("Failed to create stream: {:?}", e);
                    set_is_loading(false);
                    return;
                }
            };

            let resp = resp_value.dyn_into::<web_sys::Response>().unwrap();
            let json = match JsFuture::from(resp.json().unwrap()).await {
                Ok(json) => json,
                Err(e) => {
                    log::error!("Failed to parse stream response: {:?}", e);
                    set_is_loading(false);
                    return;
                }
            };

            let stream_data: StreamResponse = serde_wasm_bindgen::from_value(json).unwrap();
            let stream_id = stream_data.stream_id;
            set_current_stream_id(Some(stream_id.clone()));

            // Start SSE connection
            let url = format!(
                "/api/rag-query?stream_id={}&query={}&search_type={}",
                stream_id,
                urlencoding::encode(&query),
                match search_type.get() {
                    SearchType::OpenAISemantic => "openai",
                    SearchType::LocalSemantic => "local",
                    SearchType::Basic => "basic",
                }
            );

            let event_source = EventSource::new(&url).expect("Failed to connect to SSE endpoint");
            let event_source_clone = event_source.clone();

            let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
                if let Some(data) = event.data().as_string() {
                    match serde_json::from_str::<RagResponse>(&data) {
                        Ok(response) => match response.message_type.as_str() {
                            "status" => {
                                if let Some(status) = response.content {
                                    set_status_message(status);
                                }
                            }
                            "citations" => {
                                if let Some(citations) = response.citations {
                                    set_current_citations(citations);
                                }
                            }
                            "content" => {
                                if let Some(content) = response.content {
                                    set_current_response.update(|resp| resp.push_str(&content));
                                }
                            }
                            "error" => {
                                if let Some(error) = response.content {
                                    log::error!("RAG error: {}", error);
                                    
                                    // Add error message to chat
                                    let error_message = RagMessage {
                                        role: "assistant".to_string(),
                                        content: format!("Sorry, I encountered an error: {}", error),
                                        citations: None,
                                        timestamp: js_sys::Date::new_0().to_iso_string().as_string().unwrap(),
                                    };
                                    set_messages.update(|msgs| msgs.push(error_message));
                                }
                                set_is_loading(false);
                                set_current_stream_id(None);
                                set_status_message(String::new());
                                event_source_clone.close();
                            }
                            "done" => {
                                // Add assistant message to chat
                                let assistant_message = RagMessage {
                                    role: "assistant".to_string(),
                                    content: current_response.get(),
                                    citations: if current_citations.get().is_empty() {
                                        None
                                    } else {
                                        Some(current_citations.get())
                                    },
                                    timestamp: js_sys::Date::new_0().to_iso_string().as_string().unwrap(),
                                };

                                set_messages.update(|msgs| msgs.push(assistant_message));
                                set_current_response(String::new());
                                set_current_citations(Vec::new());
                                set_status_message(String::new());
                                set_is_loading(false);
                                set_current_stream_id(None);
                                event_source_clone.close();
                            }
                            _ => {}
                        },
                        Err(e) => {
                            log::error!("Failed to parse RAG response: {}", e);
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);

            let event_source_error = event_source.clone();
            let on_error = Closure::wrap(Box::new(move |error: ErrorEvent| {
                log::error!("SSE Error: {:?}", error);
                set_is_loading(false);
                set_current_stream_id(None);
                set_status_message(String::new());
                
                // Add error message to chat
                let error_message = RagMessage {
                    role: "assistant".to_string(),
                    content: "Sorry, I encountered a connection error. Please try again.".to_string(),
                    citations: None,
                    timestamp: js_sys::Date::new_0().to_iso_string().as_string().unwrap(),
                };
                set_messages.update(|msgs| msgs.push(error_message));
                
                event_source_error.close();
            }) as Box<dyn FnMut(_)>);

            event_source.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            event_source.set_onerror(Some(on_error.as_ref().unchecked_ref()));

            on_message.forget();
            on_error.forget();
        });
    };

    let handle_key_press = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            send_message();
        }
    };

    let clear_chat = move |_| {
        set_messages(Vec::new());
        set_current_response(String::new());
        set_current_citations(Vec::new());
        set_status_message(String::new());
        if is_loading.get() {
            cancel_current_request();
        }
    };

    view! {
        <div class="flex flex-col h-[700px] max-w-6xl mx-auto bg-white dark:bg-teal-800 rounded-lg shadow-lg">
            // Header
            <div class="flex items-center justify-between p-4 border-b border-gray-200 dark:border-teal-700">
                <h2 class="text-xl font-semibold text-gray-800 dark:text-gray-200">
                    "RAG Chat - Ask about blog posts"
                </h2>
                <div class="flex items-center space-x-3">
                    <select
                        class="px-3 py-1 text-sm rounded-md bg-gray-100 dark:bg-teal-700 
                        text-gray-700 dark:text-gray-200 border border-gray-300 dark:border-teal-600"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            let new_type = match value.as_str() {
                                "openai" => SearchType::OpenAISemantic,
                                "local" => SearchType::LocalSemantic,
                                _ => SearchType::OpenAISemantic,
                            };
                            set_search_type(new_type);
                        }
                    >
                        <option value="openai">"OpenAI Search"</option>
                        <option value="local">"Local Search"</option>
                    </select>
                    <button
                        class="px-3 py-1 text-sm bg-gray-500 hover:bg-gray-600 text-white rounded-md transition-colors"
                        on:click=clear_chat
                    >
                        "Clear Chat"
                    </button>
                    {move || {
                        is_loading
                            .get()
                            .then(|| {
                                view! {
                                    <button
                                        class="px-3 py-1 text-sm bg-salmon-500 hover:bg-salmon-600 text-gray-300 rounded-md transition-colors"
                                        on:click=move |_| cancel_current_request()
                                    >
                                        "Cancel"
                                    </button>
                                }
                            })
                    }}
                </div>
            </div>

            // Messages area
            <div class="flex-1 overflow-y-auto p-4 space-y-4">
                {move || {
                    if messages.get().is_empty() && !is_loading.get() {
                        view! {
                            <div class="flex justify-center items-center h-full">
                                <div class="text-center text-gray-500 dark:text-gray-400">
                                    <p class="text-lg mb-2">
                                        "Ask me anything about the blog posts!"
                                    </p>
                                    <p class="text-sm">
                                        "I can search through summaries and help you find relevant information."
                                    </p>
                                </div>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <For
                                each=move || messages.get()
                                key=|msg| format!("{}_{}", msg.timestamp, msg.role)
                                children=move |message| {
                                    view! { <MessageBubble message=message /> }
                                }
                            />
                        }
                            .into_any()
                    }
                }} // Current response being streamed
                {move || {
                    let current = current_response.get();
                    let citations = current_citations.get();
                    let status = status_message.get();
                    if is_loading.get()
                        && (!current.is_empty() || !citations.is_empty() || !status.is_empty())
                    {
                        view! {
                            <div class="w-full">
                                <div class="w-full max-w-none bg-gray-100 dark:bg-teal-700 rounded-lg p-4">
                                    {(!status.is_empty())
                                        .then(|| {
                                            view! {
                                                <div class="text-sm text-gray-500 dark:text-gray-400 mb-3 italic flex items-center">
                                                    <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-500 mr-2"></div>
                                                    {status}
                                                </div>
                                            }
                                        })}
                                    {(!citations.is_empty())
                                        .then(|| view! { <CitationsList citations=citations /> })}
                                    {(!current.is_empty())
                                        .then(|| {
                                            view! {
                                                <div class="text-gray-800 dark:text-gray-200 w-full">
                                                    <MarkdownRenderer content=current class="" />
                                                    <span class="animate-pulse text-seafoam-600 dark:text-seafoam-400">
                                                        "|"
                                                    </span>
                                                </div>
                                            }
                                        })}
                                </div>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
            </div>

            // Input area
            <div class="p-4 border-t border-gray-200 dark:border-teal-700">
                <div class="flex space-x-3">
                    <textarea
                        class="flex-1 p-3 border border-gray-300 dark:border-teal-600 rounded-lg 
                        bg-white dark:bg-teal-700 text-gray-800 dark:text-gray-200
                        focus:outline-none focus:ring-2 focus:ring-seafoam-500 dark:focus:ring-aqua-400
                        resize-none placeholder-gray-400 dark:placeholder-gray-500"
                        placeholder="Ask a question about the blog posts... (Press Enter to send, Shift+Enter for new line)"
                        rows="3"
                        prop:value=current_input
                        on:input=move |ev| set_current_input(event_target_value(&ev))
                        on:keydown=handle_key_press
                        prop:disabled=is_loading
                    ></textarea>
                    <button
                        class="px-6 py-2 bg-seafoam-600 dark:bg-seafoam-500 text-white rounded-lg
                        hover:bg-seafoam-700 dark:hover:bg-seafoam-600 transition-colors
                        disabled:bg-gray-400 dark:disabled:bg-gray-600 disabled:cursor-not-allowed
                        flex items-center justify-center min-w-[80px]"
                        on:click=move |_| send_message()
                        prop:disabled=move || {
                            is_loading.get() || current_input.get().trim().is_empty()
                        }
                    >
                        {move || {
                            if is_loading.get() {
                                view! {
                                    <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                                }
                                    .into_any()
                            } else {
                                view! { <span>"Send"</span> }.into_any()
                            }
                        }}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
fn MessageBubble(message: RagMessage) -> impl IntoView {
    let is_user = message.role == "user";
    
    view! {
        <div class="w-full">
            <div class=format!(
                "w-full rounded-lg p-4 {}",
                if is_user {
                    "bg-seafoam-600 dark:bg-seafoam-500 text-white ml-auto max-w-4xl"
                } else {
                    "bg-gray-100 dark:bg-teal-700 text-gray-800 dark:text-gray-200 max-w-none"
                },
            )>
                {move || {
                    if is_user {
                        view! {
                            <div class="whitespace-pre-wrap text-left">
                                {message.content.clone()}
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="w-full text-left">
                                <MarkdownRenderer content=message.content.clone() class="" />
                            </div>
                        }
                            .into_any()
                    }
                }}
                {message.citations.map(|citations| view! { <CitationsList citations=citations /> })}

                <div class="text-xs opacity-70 mt-2 text-left">
                    {format_timestamp(&message.timestamp)}
                </div>
            </div>
        </div>
    }
}

#[component]
fn CitationsList(citations: Vec<Citation>) -> impl IntoView {
    view! {
        <div class="mt-4 pt-3 border-t border-gray-300 dark:border-teal-600">
            <div class="text-sm font-medium text-gray-600 dark:text-gray-300 mb-3 text-left">
                {format!("Sources ({}):", citations.len())}
            </div>
            <div class="grid gap-2 max-h-48 overflow-y-auto">
                <For
                    each=move || citations.clone()
                    key=|citation| citation.link.clone()
                    children=move |citation| {
                        view! {
                            <div class="text-xs bg-white dark:bg-teal-800 rounded-md p-3 border border-gray-200 dark:border-teal-600 hover:border-seafoam-400 dark:hover:border-seafoam-500 transition-colors text-left">
                                <a
                                    href=citation.link.clone()
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    class="font-medium text-seafoam-600 dark:text-seafoam-400 hover:underline block mb-1"
                                >
                                    {citation.title}
                                </a>
                                <div class="text-gray-500 dark:text-gray-400 flex justify-between items-center">
                                    <span>
                                        {format!(
                                            "{} • {}",
                                            citation.company,
                                            citation.published_at,
                                        )}
                                    </span>
                                    <span class="text-xs bg-gray-100 dark:bg-teal-700 px-2 py-0.5 rounded">
                                        {format!("{:.1}%", citation.relevance_score * 100.0)}
                                    </span>
                                </div>
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}

fn format_timestamp(timestamp: &str) -> String {
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(timestamp));
    date.to_locale_string("en-US", &js_sys::Object::new()).as_string().unwrap_or_else(|| timestamp.to_string())
}

