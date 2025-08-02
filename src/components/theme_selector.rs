use cfg_if::cfg_if;
use leptos::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Theme {
    Neon,
    Royal,
    Earth,
    Summer,
    Army,
    Peach,
}

impl Theme {
    fn as_str(&self) -> &'static str {
        match self {
            Theme::Neon => "neon",
            Theme::Royal => "royal",
            Theme::Earth => "earth",
            Theme::Summer => "summer",
            Theme::Army => "army",
            Theme::Peach => "peach",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "royal" => Theme::Royal,
            "earth" => Theme::Earth,
            "summer" => Theme::Summer,
            "army" => Theme::Army,
            "peach" => Theme::Peach,
            _ => Theme::Neon,
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            Theme::Neon => "Neon",
            Theme::Royal => "Royal",
            Theme::Earth => "Earth",
            Theme::Summer => "Summer",
            Theme::Army => "Army",
            Theme::Peach => "Peach",
        }
    }
}

fn get_stored_theme() -> Theme {
    cfg_if! {
        if #[cfg(feature = "hydrate")] {
            use web_sys::window;
            if let Some(window) = window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    if let Ok(Some(theme_str)) = storage.get_item("selected_theme") {
                        return Theme::from_str(&theme_str);
                    }
                }
            }
            Theme::Neon
        } else {
            Theme::Neon
        }
    }
}

fn set_stored_theme(theme: &Theme) {
    cfg_if! {
        if #[cfg(feature = "hydrate")] {
            use web_sys::window;
            if let Some(storage) = window()
                .and_then(|w| w.local_storage().ok().flatten())
            {
                let _ = storage.set_item("selected_theme", theme.as_str());
            }
        }
    }
}

fn apply_theme_to_document(theme: &Theme) {
    cfg_if! {
        if #[cfg(feature = "hydrate")] {
            use web_sys::window;
            if let Some(document) = window().and_then(|w| w.document()) {
                if let Some(html_element) = document.document_element() {
                    match theme {
                        Theme::Neon => { let _ = html_element.remove_attribute("data-theme"); }
                        Theme::Royal => { let _ = html_element.set_attribute("data-theme", "royal"); }
                        Theme::Earth => { let _ = html_element.set_attribute("data-theme", "earth"); }
                        Theme::Summer => { let _ = html_element.set_attribute("data-theme", "summer"); }
                        Theme::Army => { let _ = html_element.set_attribute("data-theme", "army"); }
                        Theme::Peach => { let _ = html_element.set_attribute("data-theme", "peach"); }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ThemeSelector() -> impl IntoView {
    let (current_theme, set_current_theme) = signal(Theme::Neon);

    cfg_if! {
        if #[cfg(feature = "hydrate")] {
            Effect::new(move |_| {
                set_current_theme(get_stored_theme());
            });

            Effect::new(move |_| {
                let theme = current_theme.get();
                apply_theme_to_document(&theme);
                set_stored_theme(&theme);
            });
        }
    }

    let change_theme = move |new_theme: Theme| {
        set_current_theme(new_theme);
    };

    view! {
        <div class="flex items-center space-x-2">
            <label class="text-sm font-medium text-gray-700 dark:text-gray-300">"Theme:"</label>
            <select
                class="px-2 py-1 text-sm rounded border border-gray-300 dark:border-gray-600
                bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
                on:change=move |ev| {
                    let value = event_target_value(&ev);
                    change_theme(Theme::from_str(&value));
                }
                prop:value=move || current_theme.get().as_str()
            >
                <option value="neon">{Theme::Neon.display_name()}</option>
                <option value="royal">{Theme::Royal.display_name()}</option>
                <option value="earth">{Theme::Earth.display_name()}</option>
                <option value="summer">{Theme::Summer.display_name()}</option>
                <option value="army">{Theme::Army.display_name()}</option>
                <option value="peach">{Theme::Peach.display_name()}</option>
            </select>
        </div>
    }
}
