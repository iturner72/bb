use leptos::*;
use super::api::{AdminLoginFn, verify_token};

#[component]
pub fn AdminLogin() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (error, set_error) = create_signal(Option::<String>::None);
    
    let login_action = create_server_action::<AdminLoginFn>();
    
    create_effect(move |_| {
        if let Some(Ok(auth_response)) = login_action.value().get() {
            if let Ok(Some(storage)) = window().local_storage() {
                let _ = storage.set_item("auth_token", &auth_response.token);
                let _ = window().location().set_href("/rss-test");
            }
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
pub fn ProtectedRoute<F, C>(
    fallback: F,
    children: C,
) -> impl IntoView 
where
    F: Fn() -> View + 'static,
    C: Fn() -> View + 'static,
{
    let (is_authenticated, set_is_authenticated) = create_signal(false);
    
    create_effect(move |_| {
        if let Ok(Some(storage)) = window().local_storage() {
            if let Ok(Some(token)) = storage.get_item("auth_token") {
                spawn_local(async move {
                    if let Ok(is_valid) = verify_token(token).await {
                        set_is_authenticated.set(is_valid);
                        if !is_valid {
                            let _ = storage.remove_item("auth_token");
                        }
                    }
                });
            }
        }
    });

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
