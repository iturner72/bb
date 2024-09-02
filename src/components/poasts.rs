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

    #[derive(Debug)]
    enum PoastError {
        RequestError(String),
        JsonParseError(String),
    }
    
    impl fmt::Display for PoastError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                PoastError::RequestError(e) => write!(f, "Request error: {}", e),
                PoastError::JsonParseError(e) => write!(f, "JSON parse error: {}", e),
            }
        }
    }
    
    fn to_server_error(e: PoastError) -> ServerFnError {
        ServerFnError::ServerError(e.to_string())
    }

    info!("Fetching blog poasts from Supabase...");
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
            error!("Supabase request error: {}", e);
            PoastError::RequestError(e.to_string())
        }).map_err(to_server_error)?;

    info!("Received response from Supabase");
    info!("Response status: {:?}", response.status());
    
    let body = response.text().await.map_err(|e| {
        error!("Error reading response body: {}", e);
        PoastError::RequestError(e.to_string())
    }).map_err(to_server_error)?;

    info!("Response body length: {}", body.len());
//    info!("Response body (first 2800 chars): {}", body.chars().take(2800).collect::<String>());

    if body.trim().is_empty() {
        error!("Empty response from Supabase");
        return Err(ServerFnError::ServerError("Empty response from Supabase".to_string()));
    }

    let poasts: Vec<Poast> = from_str(&body).map_err(|e| {
        error!("JSON parse error: {}. Body: {}", e, body);
        PoastError::JsonParseError(format!("Failed to parse JSON: {}", e))
    }).map_err(to_server_error)?;

    info!("Successfully parsed {} poasts", poasts.len());

    Ok(poasts)
}

#[component]
pub fn Poasts() -> impl IntoView {
    let poasts = create_resource(|| (), |_| get_poasts());

    view! {
        <div class="space-y-8">
            <Suspense fallback=|| view! { <p class="text-center text-mint-700">"chill..."</p> }>
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
                                        <p class="text-salmon-400">"Error loading poasts: " {e.to_string()}</p> 
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
            class="relative"
            on:mouseenter=move |_| set_show_details(true)
            on:mouseleave=move |_| set_show_details(false)
        >
            <a 
                href={poast.link.clone()}
                class="block mb-24"
            >
                <article class="flex flex-col items-center cursor-pointer h-72 w-96 bg-gray-400 border-4 border-gray-700 hover:border-gray-800 p-6 shadow-lg hover:shadow-xl transition-all duration-0">
                    <div class="flex items-center pb-2">
                        {poast.links.clone().and_then(|links| links.logo_url).map(|url| view! {
                            <img src={url} alt={format!("{} logo", poast.company)} class="w-10 h-10 mr-2 rounded-sm" />
                        })}
                        <h2 class="ib text-2xl text-aqua-600">{&poast.company}</h2>
                    </div>
                    <p class="text-mint-900">
                        <span class="ib text-base text-teal-600">{&poast.title}</span>
                        " • "
                        {&poast.published_at}
                    </p>
                    <div class="poast-summary">
                        {poast.summary.clone().map(|summary| view! {
                            <p class="mb-4 text-teal-400">{summary}</p>
                        })}
                    </div>
                </article>
            </a>
            {move || if show_details.get() {
                view! {
                    <>
                    <div class="poast-details absolute top-1/2 left-1/3 ml-[-2] h-auto w-72 bg-gray-600 p-4 shadow-lg rounded-sm overflow-y-auto transform -translate-y-2/3 md:transform-none md:top-10 md:left-1/2">
                        {
                            if let Some(full_text) = poast.full_text.clone() {
                                view! { <p class="ii text-xs text-aqua-600">{full_text}</p> }
                            } else if let Some(description) = poast.description.clone() {
                                view! { <p class="ii text-xs text-wenge-400">{description}</p> }
                            } else {
                                view! { <p class="ii text-xs text-gray-400">"No details available"</p> }
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
