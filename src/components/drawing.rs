use crate::components::auth_nav::AuthNav;
use crate::components::footer::Footer;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

#[component]
pub fn DrawingPage() -> impl IntoView {
    let navigate = use_navigate();

    // Auto-redirect to rooms page
    Effect::new(move |_| {
        navigate("/rooms", Default::default());
    });

    view! {
        <div class="w-full mx-auto pl-2 bg-gray-100 dark:bg-teal-900 min-h-screen">
            <div class="flex justify-between items-center">
                <a
                    href="/"
                    class="text-3xl text-left text-seafoam-600 dark:text-mint-400 ib pl-4 p-4 font-bold"
                >
                    "bryptoblogs"
                </a>
                <AuthNav />
            </div>

            <div class="container mx-auto py-6 flex flex-col items-center justify-center min-h-[60vh]">
                <div class="text-center space-y-4">
                    <h1 class="text-2xl font-bold text-gray-800 dark:text-gray-200">
                        "Redirecting to Drawing Rooms..."
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400">
                        "Drawing is now organized by rooms for better collaboration!"
                    </p>
                    <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-seafoam-600 mx-auto"></div>

                    <div class="pt-4">
                        <a
                            href="/rooms"
                            class="bg-seafoam-600 hover:bg-seafoam-700 text-white font-bold py-2 px-4 rounded transition-colors"
                        >
                            "Go to Drawing Rooms"
                        </a>
                    </div>
                </div>
            </div>

            <Footer />
        </div>
    }
}
