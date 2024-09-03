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
                PoastError::RequestError(e) => write!(f, "reqwest error: {}", e),
                PoastError::JsonParseError(e) => write!(f, "JSON parse error: {}", e),
            }
        }
    }
    
    fn to_server_error(e: PoastError) -> ServerFnError {
        ServerFnError::ServerError(e.to_string())
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

    Ok(poasts)
}

#[component]
pub fn Poasts() -> impl IntoView {
    let poasts = create_resource(|| (), |_| get_poasts());

    view! {
        <div class="space-y-4">
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
                                        <p class="text-salmon-400">"error loading poasts: " {e.to_string()}</p> 
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
                <article class="base-poast flex flex-col items-start cursor-pointer h-full w-full bg-gray-400 border-4 border-gray-700 hover:border-gray-800 p-4 shadow-lg hover:shadow-xl transition-all duration-0">
                    <div class="flex items-center pb-2 max-w-1/2">
                        {poast.links.clone().and_then(|links| links.logo_url).map(|url| view! {
                            <img src={url} alt={format!("{} logo", poast.company)} class="w-8 h-8 mr-2 rounded-sm" />
                        })}
                        <h2 class="ib text-lg md:text-xl lg:text-2xl text-aqua-600 truncate">{&poast.company}</h2>
                    </div>
                    <div class="flex flex-col w-full space-y-0">
                        <p class="text-sm md:text-base lg:text-lg text-mint-900 truncate">
                            <span class="ib text-sm md:text-base lg:text-lg text-teal-600">{&poast.title}</span>
                        </p>
                        " • "
                        <p class="text-xs md:text-sm lg:text-base text-mint-900">{&poast.published_at}</p>
                    </div>
                    <div class="poast-summary mt-2 w-full">
                        {poast.summary.clone().map(|summary| view! {
                            <p class="text-sm md:text-base lg:text-lg text-teal-400 line-clamp-3">{summary}</p>
                        })}
                    </div>
                </article>
            </a>
            {move || if show_details.get() {
                view! {
                    <>
                    <div class="poast-details absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 max-h-72 md:max-h-44 max-w-7/12 bg-gray-600 p-4 shadow-lg rounded-sm overflow-y-auto z-10">
                        {
                            if let Some(full_text) = poast.full_text.clone() {
                                view! { 
                                    <>
                                        <p class="ii text-sm text-aqua-600">{full_text}</p> 
                                    </>
                                }
                            } else if let Some(description) = poast.description.clone() {
                                view! { 
                                    <>
                                        <div class="ii text-sm text-aqua-600" inner_html={description}></div>
                                    </>
                                }
                            } else {
                                view! {
                                    <>
                                        <p class="ii text-sm text-gray-400">"no details available"</p> 
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
