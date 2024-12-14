use leptos::prelude::*;
use serde::{Serialize, Deserialize};
use std::borrow::Cow;
use crate::components::search::BlogSearch;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Poast {
    pub id: i32,
    pub published_at: String,
    pub company: String,
    pub title: String,
    pub link: String,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub full_text: Option<String>,
    pub links: Option<Links>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Links {
    pub logo_url: Option<String>,
}

#[server(GetPoasts, "/api")]
pub async fn get_poasts(search_term: Option<String>) -> Result<Vec<Poast>, ServerFnError> {
    use crate::supabase::get_client;
    use serde_json::from_str;
    use std::fmt;
    use log::{info, error};
    use std::time::Instant;
    use crate::server_fn::cache::{POASTS_CACHE, CACHE_DURATION};

    #[derive(Debug)]
    enum PoastError {
        RequestError(String),
        JsonParseError(String),
    }
    
    impl fmt::Display for PoastError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                PoastError::RequestError(e) => write!(f, "reqwest error: {}", e),
                PoastError::JsonParseError(e) => write!(f, "JSON parse error: {}", e),
            }
        }
    }
    
    fn to_server_error(e: PoastError) -> ServerFnError {
        ServerFnError::ServerError(e.to_string())
    }

    // check cache only if no search term is provided
    if search_term.is_none() {
        let cache_duration = CACHE_DURATION;
        let cached_data = POASTS_CACHE.lock().unwrap().clone();

        if let (Some(cached_poasts), last_fetch) = cached_data {
            if last_fetch.elapsed() < cache_duration {
                info!("Returning cached poasts");
                return Ok(cached_poasts);
            }
        }
    }

    info!("fetching blog poasts from supabase...");
    let client = get_client();

    let mut request = client
        .from("poasts")
        .select("id, published_at, company, title, link, summary, links!posts_company_fkey(logo_url)")
        .order("published_at.desc")
        .limit(30);

    if let Some(ref term) = search_term {
        if !term.trim().is_empty() {
            info!("Searching for term: {}", term);
            request = request.or(format!(
                "title.ilike.%{}%,summary.ilike.%{}%",
                term, term
            ));
        }
    }

    let response = request
        .execute()
        .await
        .map_err(|e| {
            error!("supabase request error: {}", e);
            PoastError::RequestError(e.to_string())
        }).map_err(to_server_error)?;

    info!("received response from Supabase");
    info!("response status: {:?}", response.status());
    
    let body = response.text().await.map_err(|e| {
        error!("error reading response body: {}", e);
        PoastError::RequestError(e.to_string())
    }).map_err(to_server_error)?;

    info!("response body length: {}", body.len());

    if body.trim().is_empty() {
        error!("empty response from Supabase");
        return Err(ServerFnError::ServerError("empty response from Supabase".to_string()));
    }

    let poasts: Vec<Poast> = from_str(&body).map_err(|e| {
        error!("JSON parse error: {}. Body: {}", e, body);
        PoastError::JsonParseError(format!("failed to parse JSON: {}", e))
    }).map_err(to_server_error)?;

    info!("successfully parsed {} poasts", poasts.len());

    // update cache
    if search_term.is_none() {
        let mut cache = POASTS_CACHE.lock().unwrap();
        *cache = (Some(poasts.clone()), Instant::now());
    }

    Ok(poasts)
}

#[component]
pub fn Poasts() -> impl IntoView {
    let (search_input, set_search_input) = signal(String::new());
    
    let search_term = Memo::new(move |_| {
        Some(search_input.get())
    });
    
    let poasts = Resource::new(
        move || search_term.get(),
        get_poasts
    );

    let (selected_company, set_selected_company) = signal(String::new());
    let (filtered_poasts, set_filtered_poasts) = signal(vec![]);

    let get_unique_companies = move |posts: &[Poast]| -> Vec<String> {
        let mut companies: Vec<String> = posts
            .iter()
            .map(|p| p.company.clone())
            .collect();
        companies.sort();
        companies.dedup();
        companies
    };

    Effect::new(move |_| {
        let company = selected_company().to_string();
        let _ = search_term.get();

        poasts.with(|poasts_result| {
            match poasts_result {
                Some(Ok(poasts)) => {
                    let filtered = if company.is_empty() {
                        poasts.clone()
                    } else {
                        poasts.iter()
                            .filter(|poast| poast.company == company)
                            .cloned()
                            .collect()
                    };
                    set_filtered_poasts(filtered);
                },
                _ => set_filtered_poasts(vec![]),
            }
        });
    });

    view! {
        <div class="pt-4 space-y-4">
            <BlogSearch on_search=set_search_input/>

            <Suspense fallback=|| view! { <div class="pl-4 h-10"></div> }>
                <div class="flex justify-start mb-2 pl-4">
                    {move || {
                        poasts.get().map(|posts_result| {
                            match posts_result {
                                Ok(posts) => {
                                    let companies = get_unique_companies(&posts);
                                    view! {
                                        <>
                                            <select
                                                on:change=move |ev| set_selected_company(event_target_value(&ev))
                                                class="w-52 p-2 rounded-md bg-gray-100 dark:bg-teal-800 text-gray-800 dark:text-gray-200 
                                                       border border-teal-500 dark:border-seafoam-500 
                                                       focus:border-seafoam-600 dark:focus:border-aqua-400 
                                                       focus:outline-none focus:ring-2 focus:ring-seafoam-500 dark:focus:ring-aqua-400"
                                            >
                                                <option value="">"All Companies"</option>
                                                {companies.into_iter()
                                                    .map(|company| {
                                                        view! {
                                                            <option value={company.clone()}>{company.clone()}</option>
                                                        }
                                                    })
                                                    .collect_view()
                                                }
                                            </select>
                                        </>
                                    }.into_any()
                                }
                                Err(_) => view! { <><div></div></> }.into_any()
                            }
                        })
                    }}
                </div>
            </Suspense>

            <Suspense fallback=|| view! { <p class="text-center text-teal-600 dark:text-aqua-400">"Loading..."</p> }>
                {move || {
                    let poasts = filtered_poasts();
                    if poasts.is_empty() {
                        view! { <div class="text-center text-gray-500 dark:text-gray-400">"No posts found"</div> }.into_any()
                    } else {
                        view! {
                            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
                                <For
                                    each=move || poasts.clone()
                                    key=|poast| poast.id
                                    children=move |poast| view! { 
                                        <BlogPoast 
                                            poast=poast 
                                            search_term=search_input.get()
                                        /> 
                                    }
                                />
                            </div>
                        }.into_any()
                    }
                }}
            </Suspense>

        </div>
    }
}

#[component]
pub fn BlogPoast(
    poast: Poast,
    #[prop(into, optional)] search_term: String,
) -> impl IntoView {
    let company = Memo::new(move |_| poast.company.clone());
    
    view! {
        <div class="relative p-4">
            <a 
                href={poast.link.clone()}
                class="block"
                target="_blank"
                rel="noopener noreferrer"
            >
                <article class="base-poast flex flex-col items-start cursor-pointer h-full w-full bg-white dark:bg-teal-800 border-2 border-gray-200 dark:border-teal-700 hover:border-seafoam-500 dark:hover:border-aqua-500 p-4 rounded-lg shadow-md hover:shadow-lg transition-all">
                    <div class="flex items-center pb-2 max-w-1/2">
                        {move || {
                            let company_val = company.get();
                            poast.links.clone().and_then(|links| links.logo_url).map(|url| view! {
                                <img 
                                    src={url} 
                                    alt={format!("{} logo", company_val)} 
                                    class="w-8 h-8 mr-2 rounded-sm" 
                                />
                            })
                        }}
                        <h2 class="text-sm md:text-base lg:text-lg text-teal-600 dark:text-mint-400 font-semibold truncate">
                            {move || company.get()}
                        </h2>
                    </div>
                    <div class="poast-headings flex flex-col w-full space-y-1">
                        <p class="text-sm md:text-base lg:text-lg text-gray-800 dark:text-gray-200">
                            <HighlightedText
                                text=Cow::from(poast.title.clone())
                                search_term=search_term.clone()
                                class="text-sm md:text-base lg:text-lg text-seafoam-600 dark:text-aqua-400 line-clamp-1 md:line-clamp-2 lg:line-clamp-2 font-medium"
                            />
                        </p>
                        <p class="text-xs md:text-sm lg:text-base text-gray-500 dark:text-gray-400">
                            {poast.published_at.clone()}
                        </p>
                    </div>
                    <div class="poast-summary mt-2 w-full">
                        {move || poast.summary.clone().map(|summary| view! {
                            <HighlightedText
                                text=Cow::from(summary)
                                search_term=search_term.clone()
                                class="text-xs md:text-sm lg:text-base text-gray-600 dark:text-gray-300 line-clamp-2 md:line-clamp-3 lg:line-clamp-4"
                            />
                        })}
                    </div>
                </article>
            </a>
        </div>
    }
}

// Helper function to get highlighted segments
fn get_highlighted_segments(text: &str, search_term: &str) -> Vec<(String, bool)> {
    if search_term.is_empty() {
        return vec![(text.to_string(), false)];
    }

    let search_term = search_term.to_lowercase();
    let mut result = Vec::new();
    let mut last_index = 0;
    let text_lower = text.to_lowercase();

    while let Some(start_idx) = text_lower[last_index..].find(&search_term) {
        let absolute_start = last_index + start_idx;
        let absolute_end = absolute_start + search_term.len();

        // Add non-matching segment if there is one
        if absolute_start > last_index {
            result.push((text[last_index..absolute_start].to_string(), false));
        }

        // Add matching segment (using original case from text)
        result.push((text[absolute_start..absolute_end].to_string(), true));

        last_index = absolute_end;
    }

    // Add remaining text if any
    if last_index < text.len() {
        result.push((text[last_index..].to_string(), false));
    }

    result
}

#[component]
fn HighlightedText<'a>(
    #[prop(into)] text: Cow<'a, str>,
    #[prop(into)] search_term: String,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let segments = get_highlighted_segments(&text, &search_term);

    view! {
        <span class={class}>
            {segments.into_iter().map(|(text, is_highlight)| {
                if is_highlight {
                    view! {
                        <mark class="bg-mint-400 dark:bg-mint-900 text-seafoam-900 dark:text-seafoam-200 rounded px-0.5">
                            {text}
                        </mark>
                    }.into_any()
                } else {
                    view! { 
                        <span>{text}</span> 
                    }.into_any()
                }
            }).collect_view()}
        </span>
    }
}
