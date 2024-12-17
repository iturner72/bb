use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use super::api::{AdminLoginFn, LogoutFn, verify_token};

#[component]
pub fn AdminLogin() -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let navigate = use_navigate();
    
    let login_action = ServerAction::<AdminLoginFn>::new();
    
    Effect::new(move |_| {
        if let Some(Ok(_)) = login_action.value().get() {
            navigate("/admin-panel", NavigateOptions::default());
        } else if let Some(Err(e)) = login_action.value().get() {
            set_error.set(Some(e.to_string()));
        }
    });

    view! {
        <div class="flex min-h-screen items-center justify-center bg-gray-100 dark:bg-teal-900 p-6">
            <div class="w-full max-w-md">
                <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-8">
                    <h2 class="text-2xl font-bold mb-6 text-gray-900 dark:text-gray-100">
                        "Admin Login"
                    </h2>
                    
                    <form on:submit=move |ev| {
                        ev.prevent_default();
                        login_action.dispatch(AdminLoginFn {
                            username: username.get(),
                            password: password.get()
                        });
                    }>
                        <div class="space-y-4">
                            <div>
                                <label
                                    for="username"
                                    class="block text-sm font-medium text-gray-700 dark:text-gray-300"
                                >
                                    "Username"
                                </label>
                                <input
                                    type="text"
                                    id="username"
                                    class="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600 
                                           bg-white dark:bg-gray-700 px-3 py-2 text-sm text-gray-900 dark:text-gray-100
                                           focus:border-seafoam-500 dark:focus:border-seafoam-400 focus:outline-none
                                           focus:ring-1 focus:ring-seafoam-500 dark:focus:ring-seafoam-400"
                                    on:input=move |ev| set_username(event_target_value(&ev))
                                    prop:value=username
                                />
                            </div>
                            
                            <div>
                                <label
                                    for="password"
                                    class="block text-sm font-medium text-gray-700 dark:text-gray-300"
                                >
                                    "Password"
                                </label>
                                <input
                                    type="password"
                                    id="password"
                                    class="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600
                                           bg-white dark:bg-gray-700 px-3 py-2 text-sm text-gray-900 dark:text-gray-100
                                           focus:border-seafoam-500 dark:focus:border-seafoam-400 focus:outline-none
                                           focus:ring-1 focus:ring-seafoam-500 dark:focus:ring-seafoam-400"
                                    on:input=move |ev| set_password(event_target_value(&ev))
                                    prop:value=password
                                />
                            </div>

                            {move || error.get().map(|err| view! {
                                <div class="mt-2 text-sm text-red-600 dark:text-red-400">
                                    {err}
                                </div>
                            })}

                            <button
                                type="submit"
                                class="w-full rounded-md bg-seafoam-600 dark:bg-seafoam-500 px-4 py-2 text-sm
                                       font-medium text-white hover:bg-seafoam-700 dark:hover:bg-seafoam-600
                                       focus:outline-none focus:ring-2 focus:ring-seafoam-500 dark:focus:ring-seafoam-400
                                       focus:ring-offset-2 disabled:opacity-50"
                                prop:disabled=login_action.pending()
                            >
                                {move || if login_action.pending().get() {
                                    "Logging in..."
                                } else {
                                    "Log in"
                                }}
                            </button>
                        </div>
                    </form>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn LogoutButton() -> impl IntoView {
    let navigate = use_navigate();

    let logout_action = ServerAction::<LogoutFn>::new();

    Effect::new(move |_| {
        if logout_action.version().get() > 0 {
            navigate("/admin", NavigateOptions::default());
        }
    });

    view! {
        <button
            class="px-4 py-2 text-white bg-teal-500 rounded hover-bg-team-700"
            on:click=move |_| {
                logout_action.dispatch(LogoutFn {});
            }
        >
            "Logout"
        </button>
    }
}

#[component]
pub fn ProtectedRoute<F, C>(
    fallback: F,
    children: C,
) -> impl IntoView 
where
    F: Fn() -> AnyView + Send + 'static,
    C: Fn() -> AnyView + Send + 'static,
{
    let (is_authenticated, set_is_authenticated) = signal(false);
    let navigate = use_navigate();

    let check_auth = move || {
        let navigate = navigate.clone();
        spawn_local(async move {
            match verify_token().await {
                Ok(is_valid) => {
                    set_is_authenticated.set(is_valid);
                    if !is_valid {
                        navigate("/admin", NavigateOptions::default());
                    }
                }
                Err(_) => {
                    set_is_authenticated.set(false);
                    navigate("/admin", NavigateOptions::default());
                }
            }
        });
    };

    check_auth();

    Effect::new(move |_| check_auth());
    
    view! {
        {move || {
            if is_authenticated.get() {
                children()
            } else {
                fallback()
            }
        }}
    }
}
