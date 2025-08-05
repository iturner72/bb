use leptos::{prelude::*, task::spawn_local};

use crate::auth::{verify_token, context::AuthContext};
use crate::components::dark_mode_toggle::DarkModeToggle;
use crate::components::theme_selector::ThemeSelector;
use crate::components::user_avatar::{UserAvatar, AvatarSize};

#[component]
pub fn AuthNav() -> impl IntoView {
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
        <div class="items-end pr-4 flex space-x-4">
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
                                <a href="/admin-panel" class="hover:opacity-80 transition-opacity">
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
    }
}
