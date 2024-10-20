use crate::error_template::{AppError, ErrorTemplate};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use crate::components::poasts::Poasts;
use crate::components::dark_mode_toggle::DarkModeToggle;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {


        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/bb.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
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
                <h1 class="text-3xl text-left text-seafoam-600 dark:text-mint-400 ib pl-4 p-4 font-bold">"bryptoblogs"</h1>
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
