#[cfg(feature = "ssr")]
pub mod rag;

#[cfg(not(feature = "ssr"))]
pub mod rag {
    // Client-side stubs for types that need to be shared
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct RagMessage {
        pub role: String,
        pub content: String,
        pub citations: Option<Vec<Citation>>,
        pub timestamp: String,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Citation {
        pub title: String,
        pub company: String,
        pub link: String,
        pub published_at: String,
        pub relevance_score: f32,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct RagResponse {
        pub message_type: String,
        pub content: Option<String>,
        pub citations: Option<Vec<Citation>>,
    }
}
