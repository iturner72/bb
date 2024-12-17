use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{FlatRoutes, Route, Router, A},
    path
};

use crate::components::poasts::Poasts;
use crate::components::rss_test::RssTest;
use crate::components::batch_processor::BatchProcessor;
use crate::components::dark_mode_toggle::DarkModeToggle;
use crate::auth::auth_components::{AdminLogin, ProtectedRoute};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
          <head>
              <meta charset="utf-8" />
              <meta name="viewport" content="width=device-width, initial-scale=1" />
              <AutoReload options=options.clone() />
              <HydrationScripts options/>
              <link rel="stylesheet" id="leptos" href="/pkg/bb.css"/>
              <link rel="shortcut icon" type="image/ico" href="/favicon.ico"/>
              <MetaTags/>
          </head>
          <body>
              <App/>
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
               <Route path=path!("") view=HomePage/>
               <Route path=path!("admin") view=AdminLogin/>
               <Route path=path!("admin-panel") view=ProtectedAdminPanel/>
            </FlatRoutes>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <div class="w-full mx-auto pl-2 bg-gray-100 dark:bg-teal-900">
            <div class="flex justify-between items-center">
                <h1 class="text-3xl text-left text-seafoam-600 dark:text-mint-400 ib pl-4 p-4 font-bold">
                    "bryptoblogs"
                </h1>
                <div class="items-end pr-4 flex space-x-4">
                    <a 
                        href="/admin-panel"
                        class="text-teal-600 dark:text-aqua-400 hover:text-teal-700 dark:hover:text-aqua-300 transition-colors duration-200"
                    >
                        "admin"
                    </a>
                    <a 
                        href="https://github.com/iturner72/bb" 
                        class="text-teal-600 dark:text-aqua-400 hover:text-teal-700 dark:hover:text-aqua-300 transition-colors duration-200"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        "github"
                    </a>
                    <DarkModeToggle />
                </div>
            </div>
            <Poasts/>
        </div>
    }
}

#[component]
fn ProtectedAdminPanel() -> impl IntoView {
 view! {
        <ProtectedRoute
            fallback=move || view! {
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
            }.into_any()
            children=move || view! {
                <div class="max-w-7xl mx-auto px-4 py-6 space-y-8">
                    <div>
                        <h2 class="text-2xl font-bold text-gray-800 dark:text-gray-200 mb-4">
                            "RSS Feed Processing"
                        </h2>
                        <RssTest/>
                    </div>
                    <div>
                        <h2 class="text-2xl font-bold text-gray-800 dark:text-gray-200 mb-4">
                            "Backfill Missing Data"
                        </h2>
                        <BatchProcessor/>
                    </div>
                </div>
            }.into_any()
        />
    }
}
