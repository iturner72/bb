// src/server_fn/rss.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RssProgressUpdate {
    pub company: String,
    pub status: String,
    pub new_posts: i32,
    pub skipped_posts: i32,
    pub current_post: Option<String>, // Add title of current post being processed
}
