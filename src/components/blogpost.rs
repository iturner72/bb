use leptos::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub company: String,
    pub published_at: String,
    pub link: String,
    pub summary: String,
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
                <h2 class="text-2xl ib mb-2 text-teal-400">{&post.title}</h2>
                <p class="text-sm mb-2 ir text-mint-900">
                    <span class="font-semibold">{&post.company}</span>
                    " • "
                    {&post.published_at}
                </p>
                <p class="mb-4 ir text-gray-900">{&post.summary}</p>
            </article>
        </a>
    }
}
