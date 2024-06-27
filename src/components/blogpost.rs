use leptos::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Post {
    pub id: i32,
    pub published_at: String,
    pub company: String,
    pub title: String,
    pub link: String,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub full_text: Option<String>,
    #[serde(rename = "logo_url")]
    pub logo_url: Option<String>,
}

#[component]
pub fn BlogPost(post: Post) -> impl IntoView {
    view! {
        <a 
            href={post.link}
            target="_blank"
            rel="noopener noreferrer"
            class="block mb-6"
        >
            <article class="cursor-pointer bg-gray-500 border-4 border-gray-700 hover:border-gray-800 p-6 shadow-lg hover:shadow-xl transition-all duration-0">
                <div class="flex items-center mb-2">
                    {post.logo_url.map(|url| view! {
                        <img src={url} alt={format!("{} logo", post.company)} class="w-8 h-8 mr-2" />
                    })}
                    <h2 class="text-2xl ib text-teal-400">{&post.title}</h2>
                </div>
                <p class="text-sm mb-2 ir text-mint-900">
                    <span class="font-semibold">{&post.company}</span>
                    " • "
                    {&post.published_at}
                </p>
                {post.summary.as_ref().map(|summary| view! {
                    <p class="mb-4 ir text-gray-900">{summary}</p>
                })}
                {post.description.as_ref().map(|description| view! {
                    <p class="text-sm ir text-gray-700">{description}</p>
                })}
            </article>
        </a>
    }
}
