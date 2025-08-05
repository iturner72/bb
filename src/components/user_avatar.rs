use leptos::prelude::*;

#[derive(Clone, Debug)]
pub enum AvatarSize {
    Small,  // w-6 h-6
    Medium, // w-8 h-8
    Large,  // w-12 h-12
}

impl AvatarSize {
    pub fn classes(&self) -> &'static str {
        match self {
            AvatarSize::Small => "w-6 h-6",
            AvatarSize::Medium => "w-8 h-8",
            AvatarSize::Large => "w-12 h-12",
        }
    }

    pub fn fallback_text_size(&self) -> &'static str {
        match self {
            AvatarSize::Small => "text-xs",
            AvatarSize::Medium => "text-sm",
            AvatarSize::Large => "text-base",
        }
    }
}

#[component]
pub fn UserAvatar(
    /// Optional avatar URL
    avatar_url: Option<String>,
    /// Display name or username for fallback
    display_name: Option<String>,
    /// Size of the avatar
    #[prop(default = AvatarSize::Medium)]
    size: AvatarSize,
    /// Additional CSS classes
    #[prop(default = "".to_string())]
    class: String,
) -> impl IntoView {
    let fallback_char = display_name
        .as_ref()
        .and_then(|name| name.chars().next())
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    view! {
        <div class=format!(
            "{} rounded-full flex items-center justify-center {}",
            size.classes(),
            class,
        )>
            {if let Some(url) = avatar_url {
                view! {
                    <img
                        src=url
                        alt="User avatar"
                        class=format!("{} rounded-full object-cover", size.classes())
                    />
                }
                    .into_any()
            } else {
                view! {
                    <div class=format!(
                        "{} bg-seafoam-500 rounded-full flex items-center justify-center text-white {} font-medium",
                        size.classes(),
                        size.fallback_text_size(),
                    )>{fallback_char}</div>
                }
                    .into_any()
            }}
        </div>
    }.into_any()
}
