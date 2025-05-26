// src/local_llm_service/mod.rs
pub mod local_llm;

#[cfg(feature = "ssr")]
pub mod download_smollm;

// Re-export the download functionality
#[cfg(feature = "ssr")]
pub use download_smollm::*;
