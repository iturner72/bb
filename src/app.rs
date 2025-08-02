use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{FlatRoutes, Route, Router, A},
    hooks::use_params_map,
    path,
};
use uuid::Uuid;

use crate::auth::auth_components::{AdminLogin, ProtectedRoute};
use crate::components::auth_nav::AuthNav;
use crate::components::batch_processor::BatchProcessor;
use crate::components::drawing::DrawingPage;
use crate::components::embeddings::EmbeddingsProcessor;
use crate::components::footer::Footer;
use crate::components::local_embeddings::LocalEmbeddingsProcessor;
use crate::components::poasts::Poasts;
use crate::components::rag_chat::RagChat;
use crate::components::room_browser::RoomBrowser;
use crate::components::room_page::DrawingRoomPage;
use crate::components::rss_test::RssTest;
use crate::components::summary_refresh_processor::SummaryRefreshProcessor;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta
                    name="viewport"
                    content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no"
                />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <link rel="stylesheet" id="leptos" href="/pkg/bb.css" />
                <link rel="shortcut icon" type="image/ico" href="/favicon.ico" />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Router>
            <FlatRoutes fallback=|| "page not found.">
                <Route path=path!("") view=HomePage />
                <Route path=path!("admin") view=AdminLogin />
                <Route path=path!("admin-panel") view=ProtectedAdminPanel />
                <Route path=path!("draw") view=DrawingPage />
                <Route path=path!("rooms") view=RoomsPage />
                <Route path=path!("rooms/:room_id") view=RoomPage />
            </FlatRoutes>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <div class="w-full mx-auto pl-2 bg-gray-100 dark:bg-teal-900">
            <div class="flex justify-between items-center">
                <a

                    href="/"
                    class="text-3xl text-left text-seafoam-600 dark:text-mint-400 ib pl-4 p-4 font-bold"
                >
                    "bryptoblogs"
                </a>
                <AuthNav />
            </div>

            <div class="container mx-auto p-4 flex justify-center">
                <a

                    href="/draw"
                    class="bg-teal-500 hover:bg-teal-600 text-white font-bold py-2 px-4 rounded transition-colors"
                >
                    "Try Collaborative Drawing"
                </a>
                <a
                    href="/rooms"
                    class="bg-seafoam-500 hover:bg-seafoam-600 text-white font-bold py-2 px-4 rounded transition-colors"
                >
                    "Drawing Rooms"
                </a>
            </div>
            <Poasts />
            <Footer />
        </div>
    }
}

#[component]
fn RoomsPage() -> impl IntoView {
    view! {
        <div class="w-full min-h-screen bg-gray-100 dark:bg-teal-900">
            <div class="flex justify-between items-center p-4">
                <a
                    href="/"
                    class="text-3xl text-left text-seafoam-600 dark:text-mint-400 font-bold"
                >
                    "bryptoblogs"
                </a>
                <AuthNav />
            </div>

            <RoomBrowser />

            <Footer />
        </div>
    }
}

#[component]
fn RoomPage() -> impl IntoView {
    let params = use_params_map();

    let room_id = Memo::new(move |_| {
        params
            .get()
            .get("room_id")
            .and_then(|id_str| Uuid::parse_str(&id_str).ok())
    });

    view! { <DrawingRoomPage room_id=room_id /> }
}

#[component]
fn ProtectedAdminPanel() -> impl IntoView {
    view! {
        <ProtectedRoute
            fallback=move || {
                view! {
                    <div class="flex justify-center items-center h-[calc(100vh-5rem)]">
                        <div class="text-center">
                            <h2 class="text-xl text-gray-800 dark:text-gray-200 mb-4">
                                "Access Denied"
                            </h2>
                            <p class="text-gray-600 dark:text-gray-400 mb-4">
                                "You need to be logged in to access this page."
                            </p>
                            <A
                                href="/admin"
                                attr:class="text-seafoam-600 dark:text-seafoam-400 hover:underline"
                            >
                                "Go to Login"
                            </A>
                        </div>
                    </div>
                }
                    .into_any()
            }

            children=move || {
                view! {
                    <div class="max-w-7xl mx-auto px-4 py-6 space-y-8">
                        <div>
                            <h2 class="text-2xl font-bold text-gray-800 dark:text-gray-200 mb-4">
                                "RAG Chat"
                            </h2>
                            <RagChat />
                        </div>
                        <div>
                            <h2 class="text-2xl font-bold text-gray-800 dark:text-gray-200 mb-4">
                                "RSS Feed Processing"
                            </h2>
                            <RssTest />
                        </div>
                        <div>
                            <h2 class="text-2xl font-bold text-gray-800 dark:text-gray-200 mb-4">
                                "Generate Embeddings"
                            </h2>
                            <EmbeddingsProcessor />
                        </div>
                        <div>
                            <h2 class="text-2xl font-bold text-gray-800 dark:text-gray-200 mb-4">
                                "Generate Local Embeddings"
                            </h2>
                            <LocalEmbeddingsProcessor />
                        </div>
                        <div>
                            <h2 class="text-2xl font-bold text-gray-800 dark:text-gray-200 mb-4">
                                "Backfill Missing Data"
                            </h2>
                            <BatchProcessor />
                        </div>
                        <div>
                            <h2 class="text-2xl font-bold text-gray-800 dark:text-gray-200 mb-4">
                                "Refresh Summaries"
                            </h2>
                            <SummaryRefreshProcessor />
                        </div>
                    </div>
                }
                    .into_any()
            }
        />
    }
}
