use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{FlatRoutes, Route, Router},
    hooks::use_params_map,
    path,
};
use uuid::Uuid;

use crate::auth::auth_components::{AdminLogin, ProtectedAdminPanel};
use crate::auth::context::AuthProvider;
use crate::components::auth_nav::AuthNav;
use crate::components::drawing::DrawingPage;
use crate::components::footer::Footer;
use crate::components::poasts::Poasts;
use crate::components::room_browser::RoomBrowser;
use crate::components::room_page::DrawingRoomPage;

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
        <AuthProvider>
            <Router>
                <FlatRoutes fallback=|| "page not found.">
                    <Route path=path!("") view=HomePage />
                    <Route path=path!("admin") view=AdminLogin />
                    <Route path=path!("admin-panel") view=ProtectedAdminPanel />
                    <Route path=path!("draw") view=DrawingPage />
                    <Route path=path!("rooms") view=RoomsPage />
                    <Route path=path!("room/:room_id") view=RoomPage />
                </FlatRoutes>
            </Router>
        </AuthProvider>
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
                    href="/rooms"
                    class="bg-seafoam-500 hover:bg-seafoam-600 text-white font-bold py-2 px-4 rounded transition-colors"
                >
                    "drawing rooms"
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
