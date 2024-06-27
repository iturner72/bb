use leptos::*;
use super::blogpost::{BlogPost, Post};

#[server(GetBlogPosts)]
pub async fn get_blog_posts() -> Result<Vec<Post>, ServerFnError> {
    // TODO add supabase calls
    // dummy data for now
    Ok(vec![
        Post {
            id: 1,
            title: "Understanding Blockchain".to_string(),
            company: "CryptoTech".to_string(),
            published_at: "2023-06-01".to_string(),
            link: "https://example.com/post1".to_string(),
            summary: "An introduction to blockchain technology and its potential applications in various industries.".to_string(),
        },
        Post {
            id: 2,
            title: "The Future of DeFi".to_string(),
            company: "DeFi Innovators".to_string(),
            published_at: "2023-06-15".to_string(),
            link: "https://example.com/post2".to_string(),
            summary: "Exploring the current state and future prospects of Decentralized Finance (DeFi) in the crypto ecosystem.".to_string(),
        },
    ])
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


