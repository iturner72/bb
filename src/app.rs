use crate::error_template::{AppError, ErrorTemplate};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use crate::components::poasts::Poasts;
use crate::components::dark_mode_toggle::DarkModeToggle;
use crate::components::rss_test::RssTest;
use crate::auth::auth_components::{AdminLogin, ProtectedRoute};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/bb.css"/>
        
        <Title text="Welcome to Leptos"/>
        
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! {
                <ErrorTemplate outside_errors/>
            }
            .into_view()
        }>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                    <Route path="admin" view=AdminLogin/>
                    <Route path="rss-test" view=ProtectedRssTest/>
                </Routes>
            </main>
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
                <div class="items-end pr-4">
                    <a 
                        href="https://github.com/iturner72/bb" 
                        class="text-teal-600 dark:text-aqua-400 ib pr-4 hover:text-teal-700 dark:hover:text-aqua-300 transition-colors duration-200"
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
fn ProtectedRssTest() -> impl IntoView {
    view! {
        <ProtectedRoute
            fallback=move || view! {
                <div class="flex min-h-screen items-center justify-center bg-gray-100 dark:bg-teal-900">
                    <div class="text-center">
                        <h2 class="text-xl text-gray-800 dark:text-gray-200 mb-4">
                            "Access Denied"
                        </h2>
                        <p class="text-gray-600 dark:text-gray-400 mb-4">
                            "You need to be logged in to access this page."
                        </p>
                        <A
                            href="/admin"
                            class="text-seafoam-600 dark:text-seafoam-400 hover:underline"
                        >
                            "Go to Login"
                        </A>
                    </div>
                </div>
            }.into_view()
            children=move || view! { <RssTest/> }.into_view()
        />
    }
}
