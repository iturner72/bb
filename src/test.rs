#[deprecated(since = "0.2.0", note = "Use new JWT-based auth module instead")]
pub use legacy::*;
mod legacy {
use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthState {
    pub is_authenticated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminCredentials {
    username: String,
    password: String,
}

#[server(AdminLogin, "/api")]
pub async fn admin_login(username: String, password: String) -> Result<bool, ServerFnError> {
    use std::env;
    use dotenv::dotenv;

    // Load .env file explicitly in the server function
    dotenv().ok();

    // Add debug logging
    let admin_user = match env::var("ADMIN_USERNAME") {
        Ok(val) => {
            log::info!("Found ADMIN_USERNAME in env"); // Debug log
            val
        },
        Err(_) => {
            log::warn!("ADMIN_USERNAME not found in env, using default"); // Debug log
            "admin".to_string()
        }
    };

    let admin_pass = match env::var("ADMIN_PASSWORD") {
        Ok(val) => {
            log::info!("Found ADMIN_PASSWORD in env"); // Debug log
            val
        },
        Err(_) => {
            log::warn!("ADMIN_PASSWORD not found in env, using default"); // Debug log
            "adminpass".to_string()
        }
    };

    // Debug log the attempted login (don't log actual password in production!)
    log::info!("Login attempt - Username: {}, Expected Username: {}", username, admin_user);
    
    let auth_result = username == admin_user && password == admin_pass;
    log::info!("Auth result: {}", auth_result);

    Ok(auth_result)
}

#[component]
pub fn AdminLogin() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (error, set_error) = create_signal(Option::<String>::None);

    let login_action = create_server_action::<AdminLogin>();

    create_effect(move |_| {
        if let Some(Ok(success)) = login_action.value().get() {
            if success {
                // Store auth state in localStorage
                let _ = window()
                    .local_storage()
                    .map(|storage| storage.expect("local storage should be available").set_item("admin_authenticated", "true").ok());

                // redirect to RSS test page
                let _ = window().location().set_href("/rss-test");
            } else {
                set_error.set(Some("Invalid credentials".to_string()));
            }
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
                        login_action.dispatch(AdminLogin {
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
            if let Ok(Some(_)) = storage.get_item("admin_authenticated") {
                set_is_authenticated.set(true);
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
}

pub use crate::auth_legacy::components::*;

