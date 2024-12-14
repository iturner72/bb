use leptos::prelude::*;
use web_sys::{window, Storage};

fn get_local_storage() -> Storage {
    window()
        .expect("Failed to get window")
        .local_storage()
        .expect("Failed to get local storage")
        .expect("Local storage is not available")
}

fn get_item(key: &str) -> Option<String> {
    get_local_storage().get_item(key).ok().flatten()
}

fn set_item(key: &str, value: &str) {
    get_local_storage().set_item(key, value).expect("Failed to set item in local storage");
}

#[component]
pub fn DarkModeToggle() -> impl IntoView {
    let (is_dark, set_is_dark) = signal(false);

    Effect::new(move |_| {
        let is_dark_mode = get_item("darkMode").map(|v| v == "true").unwrap_or(false);
        set_is_dark.set(is_dark_mode);
        let window = window().expect("Failed to get window");
        let document = window.document().expect("Failed to get document");
        let body = document.body().expect("Failed to get body");
        if is_dark_mode {
            body.class_list().add_1("dark").expect("Failed to add dark class");
        } else {
            body.class_list().remove_1("dark").expect("Failed to remove dark class");
        }
    });

    let toggle_dark_mode = move |_| {
        let new_state = !is_dark.get();
        set_is_dark.set(new_state);
        set_item("darkMode", &new_state.to_string());
        let window = window().expect("Failed to get window");
        let document = window.document().expect("Failed to get document");
        let body = document.body().expect("Failed to get body");
        if new_state {
            body.class_list().add_1("dark").expect("Failed to add dark class");
        } else {
            body.class_list().remove_1("dark").expect("Failed to remove dark class");
        }
    };

    view! {
        <button
            on:click=toggle_dark_mode
        >
            {move || if is_dark.get() {
                "🌞"
            } else {
                "🌙"
            }}
        </button>
    }
}
