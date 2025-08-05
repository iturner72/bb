use leptos::{prelude::*, task::spawn_local};

use crate::auth::{verify_token, context::AuthContext};
use crate::components::dark_mode_toggle::DarkModeToggle;
use crate::components::theme_selector::ThemeSelector;
use crate::components::user_avatar::{UserAvatar, AvatarSize};

#[component]
pub fn AuthNav() -> impl IntoView {
    let (is_mobile_menu_open, set_mobile_menu_open) = signal(false);
    let (is_authenticated, set_is_authenticated) = signal(false);
    let (is_checking, set_is_checking) = signal(true);

    Effect::new(move |_| {
        spawn_local(async move {
            match verify_token().await {
                Ok(is_valid) => {
                    set_is_authenticated(is_valid);
                    set_is_checking(false);
                }
                Err(_) => {
                    set_is_authenticated(false);
                    set_is_checking(false);
                }
            }
        });
    });

    view! {
        // Mobile hamburger menu and desktop navigation
        <div class="items-end pr-2 lg:pr-0 flex lg:space-x-4">
            // Desktop navigation (hidden on mobile)
            <div class="hidden lg:flex lg:items-center lg:space-x-4">
                <a
                    href="https://github.com/iturner72/bb"
                    class="text-teal-600 dark:text-aqua-400 hover:text-teal-700 dark:hover:text-aqua-300 transition-colors duration-200"
                    target="_blank"
                    rel="noopener noreferrer"
                >
                    "github"
                </a>
                <ThemeSelector />
                <DarkModeToggle />
                {move || {
                    let auth = use_context::<AuthContext>();
                    if let Some(auth_ctx) = auth {
                        if auth_ctx.is_loading.get() {
                            view! { <span class="text-gray-400">"Loading..."</span> }.into_any()
                        } else if auth_ctx.is_authenticated.get() {
                            if let Some(user) = auth_ctx.current_user.get() {
                                view! {
                                    <a
                                        href="/admin-panel"
                                        class="hover:opacity-80 transition-opacity"
                                    >
                                        <UserAvatar
                                            avatar_url=user.avatar_url
                                            display_name=user.display_name.or(user.username)
                                            size=AvatarSize::Medium
                                        />
                                    </a>
                                }
                                    .into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        } else {
                            view! {
                                <a
                                    href="/admin"
                                    class="text-teal-600 dark:text-aqua-400 hover:text-teal-700 dark:hover:text-aqua-300 transition-colors duration-200"
                                >
                                    "Login"
                                </a>
                            }
                                .into_any()
                        }
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
            </div>

            // Mobile hamburger button (visible only on mobile)
            <button
                class="lg:hidden flex items-center justify-center w-10 h-10 text-teal-600 dark:text-aqua-400 hover:text-teal-700 dark:hover:text-aqua-300 transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-teal-500 focus:ring-opacity-50 rounded-md"
                on:click=move |_| set_mobile_menu_open.update(|open| *open = !*open)
                aria-label="Toggle mobile menu"
            >
                // Animated hamburger icon
                <div class="w-6 h-6 flex flex-col justify-center items-center">
                    <span class=move || {
                        format!(
                            "block w-5 h-0.5 bg-current transform transition-all duration-300 {}",
                            if is_mobile_menu_open.get() {
                                "rotate-45 translate-y-1"
                            } else {
                                "-translate-y-1"
                            },
                        )
                    }></span>
                    <span class=move || {
                        format!(
                            "block w-5 h-0.5 bg-current transform transition-all duration-300 {}",
                            if is_mobile_menu_open.get() { "opacity-0" } else { "opacity-100" },
                        )
                    }></span>
                    <span class=move || {
                        format!(
                            "block w-5 h-0.5 bg-current transform transition-all duration-300 {}",
                            if is_mobile_menu_open.get() {
                                "-rotate-45 -translate-y-1"
                            } else {
                                "translate-y-1"
                            },
                        )
                    }></span>
                </div>
            </button>

            // Mobile dropdown menu overlay
            {move || {
                if is_mobile_menu_open.get() {
                    view! {
                        <div class="lg:hidden">
                            // Backdrop overlay
                            <div
                                class="fixed inset-0 bg-black bg-opacity-50 z-40"
                                on:click=move |_| set_mobile_menu_open.set(false)
                            ></div>

                            // Mobile menu panel
                            <div class="fixed top-0 right-0 w-72 h-full bg-white dark:bg-teal-800 shadow-xl z-50 transform transition-transform duration-300 ease-in-out">
                                // Mobile menu header
                                <div class="flex items-center justify-between p-4 border-b border-gray-200 dark:border-teal-700">
                                    <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">
                                        "Menu"
                                    </h2>
                                    <button
                                        on:click=move |_| set_mobile_menu_open.set(false)
                                        class="p-2 text-gray-600 dark:text-gray-300 hover:text-gray-800 dark:hover:text-gray-100 hover:bg-gray-100 dark:hover:bg-teal-700 rounded-md transition-colors touch-manipulation"
                                        aria-label="Close menu"
                                    >
                                        <svg
                                            class="w-5 h-5"
                                            fill="none"
                                            stroke="currentColor"
                                            viewBox="0 0 24 24"
                                        >
                                            <path
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                                stroke-width="2"
                                                d="M6 18L18 6M6 6l12 12"
                                            />
                                        </svg>
                                    </button>
                                </div>

                                // Mobile menu content
                                <div class="flex flex-col p-4 space-y-4">
                                    // GitHub link
                                    <a
                                        href="https://github.com/iturner72/bb"
                                        class="flex items-center space-x-3 p-3 text-teal-600 dark:text-aqua-400 hover:text-teal-700 dark:hover:text-aqua-300 hover:bg-gray-100 dark:hover:bg-teal-700 rounded-md transition-colors duration-200 touch-manipulation"
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        on:click=move |_| set_mobile_menu_open.set(false)
                                    >
                                        <svg
                                            class="w-5 h-5"
                                            fill="currentColor"
                                            viewBox="0 0 24 24"
                                        >
                                            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                                        </svg>
                                        <span class="font-medium">"GitHub"</span>
                                    </a>

                                    // Theme controls section
                                    <div class="border-t border-gray-200 dark:border-teal-700 pt-4">
                                        <h3 class="text-sm font-semibold text-gray-600 dark:text-gray-400 mb-3 uppercase tracking-wide">
                                            "Appearance"
                                        </h3>
                                        <div class="space-y-3">
                                            // Theme selector
                                            <div class="flex items-center justify-between p-3 hover:bg-gray-100 dark:hover:bg-teal-700 rounded-md transition-colors">
                                                <span class="text-gray-800 dark:text-gray-200 font-medium">
                                                    "Theme"
                                                </span>
                                                <ThemeSelector />
                                            </div>

                                            // Dark mode toggle
                                            <div class="flex items-center justify-between p-3 hover:bg-gray-100 dark:hover:bg-teal-700 rounded-md transition-colors">
                                                <span class="text-gray-800 dark:text-gray-200 font-medium">
                                                    "Dark Mode Toggle"
                                                </span>
                                                <DarkModeToggle />
                                            </div>
                                        </div>
                                    </div>

                                    // User section
                                    <div class="border-t border-gray-200 dark:border-teal-700 pt-4">
                                        {move || {
                                            let auth = use_context::<AuthContext>();
                                            if let Some(auth_ctx) = auth {
                                                if auth_ctx.is_loading.get() {
                                                    view! {
                                                        <div class="flex items-center justify-center p-3">
                                                            <span class="text-gray-400">"Loading..."</span>
                                                        </div>
                                                    }
                                                        .into_any()
                                                } else if auth_ctx.is_authenticated.get() {
                                                    if let Some(user) = auth_ctx.current_user.get() {
                                                        view! {
                                                            <a
                                                                href="/admin-panel"
                                                                class="flex items-center space-x-3 p-3 hover:bg-gray-100 dark:hover:bg-teal-700 rounded-md transition-colors touch-manipulation"
                                                                on:click=move |_| set_mobile_menu_open.set(false)
                                                            >
                                                                <UserAvatar
                                                                    avatar_url=user.avatar_url
                                                                    display_name=user
                                                                        .display_name
                                                                        .clone()
                                                                        .or(user.username.clone())
                                                                    size=AvatarSize::Medium
                                                                />
                                                                <div class="flex flex-col">
                                                                    <span class="text-gray-800 dark:text-gray-200 font-medium">
                                                                        {user
                                                                            .display_name
                                                                            .or(user.username)
                                                                            .unwrap_or_else(|| "User".to_string())}
                                                                    </span>
                                                                    <span class="text-sm text-gray-500 dark:text-gray-400">
                                                                        "Admin Panel"
                                                                    </span>
                                                                </div>
                                                            </a>
                                                        }
                                                            .into_any()
                                                    } else {
                                                        view! { <div></div> }.into_any()
                                                    }
                                                } else {
                                                    view! {
                                                        <a
                                                            href="/admin"
                                                            class="flex items-center justify-center w-full p-3 bg-teal-600 dark:bg-teal-600 text-white hover:bg-teal-700 dark:hover:bg-teal-700 rounded-md transition-colors duration-200 font-medium touch-manipulation"
                                                            on:click=move |_| set_mobile_menu_open.set(false)
                                                        >
                                                            "Login"
                                                        </a>
                                                    }
                                                        .into_any()
                                                }
                                            } else {
                                                view! { <div></div> }.into_any()
                                            }
                                        }}
                                    </div>
                                </div>
                            </div>
                        </div>
                    }
                        .into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
