use leptos::*;
use serde::{Serialize, Deserialize};

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
pub async fn get_poasts() -> Result<Vec<Poast>, ServerFnError> {
    use crate::supabase::get_client;
    use serde_json::from_str;
    use std::fmt;
    use log::{info, error};
    use once_cell::sync::Lazy;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    static CACHE: Lazy<Mutex<(Option<Vec<Poast>>, Instant)>> = Lazy::new(|| Mutex::new((None, Instant::now())));
    const CACHE_DURATION: Duration = Duration::from_secs(3600);

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

    let cache_duration = CACHE_DURATION;
    let cached_data = CACHE.lock().unwrap().clone();

    // check cache
    if let (Some(cached_poasts), last_fetch) = cached_data {
        if last_fetch.elapsed() < cache_duration {
            info!("Returning cached poasts");
            info!("Cache debug: {:?}", (cached_poasts.len(), last_fetch));
            return Ok(cached_poasts);
        }
    }

    info!("fetching blog poasts from supabase...");
    let client = get_client();

    let request = client
        .from("poasts")
        .select("id, published_at, company, title, link, summary, links!posts_company_fkey(logo_url)")
        .order("published_at.desc")
        .limit(10);

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
    {
        let mut cache = CACHE.lock().unwrap();
        *cache = (Some(poasts.clone()), Instant::now());
    }

    Ok(poasts)
}

#[component]
pub fn Poasts() -> impl IntoView {
    let poasts = create_resource(|| (), |_| get_poasts());
    let (search_query, set_search_query) = create_signal(String::new());
    let (filtered_poasts, set_filtered_poasts) = create_signal(vec![]);

    create_effect(move |_| {
        let query = search_query().to_lowercase();
        poasts.with(|poasts_result| {
            match poasts_result {
                Some(Ok(poasts)) => {
                    let filtered = poasts.iter()
                        .filter(|poast| poast.company.to_lowercase().contains(&query))
                        .cloned()
                        .collect::<Vec<_>>();
                    set_filtered_poasts(filtered);
                },
                _ => set_filtered_poasts(vec![]),
            }
        });
    });

    view! {
        <div class="space-y-4">
            <div class="flex justify-start mb-4 pl-4">
                <input
                    type="text"
                    placeholder="Filter by company..."
                    on:input=move |ev| set_search_query(event_target_value(&ev))
                    class="w-1/6 p-2 rounded-md bg-gray-100 dark:bg-teal-800 text-gray-800 dark:text-gray-200 border border-teal-500 dark:border-seafoam-500 focus:border-seafoam-600 dark:focus:border-aqua-400 focus:outline-none focus:ring-2 focus:ring-seafoam-500 dark:focus:ring-aqua-400 placeholder-gray-500 dark:placeholder-gray-400"
                />
            </div>
            <Suspense fallback=|| view! { <p class="text-center text-teal-600 dark:text-aqua-400">"Loading..."</p> }>
                {move || {
                    let poasts = filtered_poasts();
                    if poasts.is_empty() {
                        view! { <div class="text-center text-gray-500 dark:text-gray-400">"No posts found"</div> }
                    } else {
                        view! {
                            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
                                <For
                                    each=move || poasts.clone()
                                    key=|poast| poast.id
                                    children=move |poast| view! { <BlogPoast poast=poast /> }
                                />
                            </div>
                        }
                    }
                }}
            </Suspense>
        </div>
    }
}

#[component]
pub fn BlogPoast(poast: Poast) -> impl IntoView {
    view! {
        <div class="relative p-4">
            <a 
                href={poast.link.clone()}
                class="block"
                target="_blank"
                rel="noopener noreferrer"
            >
                <article class="base-poast flex flex-col items-start cursor-pointer h-full w-full bg-white dark:bg-teal-800 border-2 border-gray-200 dark:border-teal-700 hover:border-seafoam-500 dark:hover:border-aqua-500 p-4 rounded-lg shadow-md hover:shadow-lg transition-all duration-300">
                    <div class="flex items-center pb-2 max-w-1/2">
                        {poast.links.clone().and_then(|links| links.logo_url).map(|url| view! {
                            <img src={url} alt={format!("{} logo", poast.company)} class="w-8 h-8 mr-2 rounded-sm" />
                        })}
                        <h2 class="text-sm md:text-base lg:text-lg text-teal-600 dark:text-mint-400 font-semibold truncate">{&poast.company}</h2>
                    </div>
                    <div class="poast-headings flex flex-col w-full space-y-1">
                        <p class="text-sm md:text-base lg:text-lg text-gray-800 dark:text-gray-200">
                            <span class="text-sm md:text-base lg:text-lg text-seafoam-600 dark:text-aqua-400 line-clamp-1 md:line-clamp-2 lg:line-clamp-2 font-medium">
                                {&poast.title}
                            </span>
                        </p>
                        <p class="text-xs md:text-sm lg:text-base text-gray-500 dark:text-gray-400">{&poast.published_at}</p>
                    </div>
                    <div class="poast-summary mt-2 w-full">
                        {poast.summary.clone().map(|summary| view! {
                            <p class="text-xs md:text-sm lg:text-base text-gray-600 dark:text-gray-300 line-clamp-2 md:line-clamp-3 lg:line-clamp-4">{summary}</p>
                        })}
                    </div>
                </article>
            </a>
        </div>
    }
}
