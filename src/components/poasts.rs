use leptos::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
        .select("*, links!posts_company_fkey(logo_url)")
        .order("published_at.desc")
        .limit(5);
    
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

    view! {
        <div class="space-y-4">
            <Suspense fallback=|| view! { <p class="text-center text-teal-100">"chill..."</p> }>
                {
                    move || {
                        poasts.get().map(|poasts_result| {
                            match poasts_result {
                                Ok(poasts) => view! {
                                    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
                                        <For
                                            each=move || poasts.clone()
                                            key=|poast| poast.id
                                            children=move |poast| view! { <BlogPoast poast=poast /> }
                                        />
                                    </div>
                                },
                                Err(e) => view! { 
                                    <div class="grid grid-cols-1 gap-3">
                                        <p class="text-salmon-600">"error loading poasts: " {e.to_string()}</p> 
                                    </div>
                                },
                            }
                        })
                    }
                }
            </Suspense>
        </div>
    }
}

#[component]
pub fn BlogPoast(poast: Poast) -> impl IntoView {
    let (show_details, set_show_details) = create_signal(false);

    view! {
        <div
            class="relative p-4"
            on:mouseenter=move |_| set_show_details(true)
            on:mouseleave=move |_| set_show_details(false)
        >
            <a 
                href={poast.link.clone()}
                class="block"
            >
                <article class="base-poast flex flex-col items-start cursor-pointer h-full w-full bg-gray-900 border-4 border-gray-700 hover:border-gray-800 p-4 shadow-lg hover:shadow-xl transition-all duration-0">
                    <div class="flex items-center pb-2 max-w-1/2">
                        {poast.links.clone().and_then(|links| links.logo_url).map(|url| view! {
                            <img src={url} alt={format!("{} logo", poast.company)} class="w-8 h-8 mr-2 rounded-sm" />
                        })}
                        <h2 class="ib text-sm md:text-base lg:text-lg text-purple-100 truncate">{&poast.company}</h2>
                    </div>
                    <div class="poast-headings flex flex-col w-full space-y-0">
                        <p class="text-sm md:text-base lg:text-lg text-mint-800">
                            <span class="ib text-sm md:text-base lg:text-lg text-teal-100 line-clamp-1 md:line-clamp-2 lg:line-clamp-2">
                                {&poast.title}
                            </span>
                        </p>
                        " • "
                        <p class="text-xs md:text-sm lg:text-base text-gray-500">{&poast.published_at}</p>
                    </div>
                    <div class="poast-summary mt-2 w-full">
                        {poast.summary.clone().map(|summary| view! {
                            <p class="text-xs md:text-sm lg:text-base text-gray-200 line-clamp-2 md:line-clamp-3 lg:line-clamp-4">{summary}</p>
                        })}
                    </div>
                </article>
            </a>
            {move || if show_details.get() {
                view! {
                    <>
                    <div class="poast-details bottom-0 max-h-96 bg-gray-800 p-4 shadow-lg rounded-sm overflow-y-auto z-10 border-teal-700">
                        {
                            if let Some(full_text) = poast.full_text.clone() {
                                view! { 
                                    <>
                                        <p class="ii text-xs text-gray-100">{full_text}</p> 
                                    </>
                                }
                            } else if let Some(description) = poast.description.clone() {
                                view! { 
                                    <>
                                        <div class="ii text-xs text-gray-100" inner_html={description}></div>
                                    </>
                                }
                            } else {
                                view! {
                                    <>
                                        <p class="ii text-base text-teal-300">"no details available"</p> 
                                    </>
                                }
                            }
                        }
                    </div>
                    </>
                }
            } else {
                view! {
                    <>
                        <div class="w-max-0"></div>
                    </>
                }
            }}
        </div>
    }
}
