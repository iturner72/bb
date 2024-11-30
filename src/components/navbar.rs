use leptos::*;
use leptos_router::A;
use crate::components::dark_mode_toggle::DarkModeToggle;

#[component]
pub fn Navbar() -> impl IntoView {
    view! {
        <div class="flex justify-between items-center bg-gray-300 dark:bg-teal-800 px-6 py-4">
            <A href="/" class="text-2xl text-teal-600 dark:text-mint-400 hover:text-teal-800 dark:hover:text-mint-300">"bryptoblogs"</A>
            <div class="items-end">
                <A href="/rss" class="pr-2 text-seafoam-600 dark:text-aqua-400 hover:text-seafoam-800 dark:hover:text-aqua-300">"rss"</A>
                <A 
                    href="https://github.com/iturner72/bb" 
                    class="text-teal-600 dark:text-aqua-400 pr-2 hover:text-teal-700 dark:hover:text-aqua-300 transition-colors"
                    target="_blank"
                >
                    "github"
                </A>
                <DarkModeToggle />
            </div>
        </div>
    }
}
