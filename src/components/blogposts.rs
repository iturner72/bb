use cfg_if::cfg_if;
use leptos::*;
use super::blogpost::{BlogPost, Post};

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use crate::supabase::get_client;
        use serde_json::from_str;
        use std::fmt;
        use log::{info, error};

        #[derive(Debug)]
        enum BlogError {
            RequestError(String),
            JsonParseError(String),
        }
        
        impl fmt::Display for BlogError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    BlogError::RequestError(e) => write!(f, "Request error: {}", e),
                    BlogError::JsonParseError(e) => write!(f, "JSON parse error: {}", e),
                }
            }
        }
        
        fn to_server_error(e: BlogError) -> ServerFnError {
            ServerFnError::ServerError(e.to_string())
        }
    } 
}


#[server(GetBlogPosts, "/api")]
pub async fn get_blog_posts() -> Result<Vec<Post>, ServerFnError> {
    info!("Fetching blog posts from Supabase...");
    let client = get_client();
    
    let request = client
        .from("posts")
        .select("*")
        .order("published_at.desc")
        .limit(10);

    let response = request
        .execute()
        .await
        .map_err(|e| {
            error!("Supabase request error: {}", e);
            BlogError::RequestError(e.to_string())
        }).map_err(to_server_error)?;

    info!("Received response from Supabase");
    info!("Response status: {:?}", response.status());
    
    let body = response.text().await.map_err(|e| {
        error!("Error reading response body: {}", e);
        BlogError::RequestError(e.to_string())
    }).map_err(to_server_error)?;

    info!("Response body length: {}", body.len());
    info!("Response body (first 100 chars): {}", body.chars().take(100).collect::<String>());

    if body.trim().is_empty() {
        error!("Empty response from Supabase");
        return Err(ServerFnError::ServerError("Empty response from Supabase".to_string()));
    }

    let posts: Vec<Post> = from_str(&body).map_err(|e| {
        error!("JSON parse error: {}. Body: {}", e, body);
        BlogError::JsonParseError(format!("Failed to parse JSON: {}", e))
    }).map_err(to_server_error)?;

    info!("Successfully parsed {} posts", posts.len());

    Ok(posts)
}


#[component]
pub fn BlogPosts() -> impl IntoView {
    let posts = create_resource(|| (), |_| get_blog_posts());

    view! {
        <div class="space-y-8">
            <Suspense fallback=move || view! { <p class="text-center ir">"Loading..."</p> }>
                {move || {
                    posts.get().map(|posts_result| {
                        match posts_result {
                            Ok(posts) => view! {
                                <div class="grid grid-cols-1 gap-6">
                                    <For
                                        each=move || posts.clone()
                                        key=|post| post.id
                                        children=move |post| view! { <BlogPost post=post /> }
                                    />
                                </div>
                            },
                            Err(e) => view! { 
                                <div class="grid grid-cols-1 gap-6">
                                    <p class="text-red-500 ir">"Error loading posts: " {e.to_string()}</p> 
                                </div>
                            },
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}
