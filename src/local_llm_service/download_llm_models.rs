//! src/local_llm_service/download_models.rs
//! Multi-model downloader for local LLM inference
//! Downloads various model files optimized for local inference

use log::info;
use std::fs;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug)]
pub enum ModelType {
    /// SmolLM2-135M (smallest, fastest)
    SmolLM2135M,
    /// SmolLM2-1.7B (medium size)  
    SmolLM21_7B,
    /// Llama 3.2-3B (best quality for RAG)
    Llama32_3B,
}

// SmolLM2-135M model files (smallest, fastest)
const SMOLLM_135M_FILES: [(&str, &str); 2] = [
    (
        "smollm_135m_model.safetensors",
        "https://huggingface.co/HuggingFaceTB/SmolLM2-135M-Instruct/resolve/main/model.safetensors",
    ),
    (
        "smollm_135m_tokenizer.json",
        "https://huggingface.co/HuggingFaceTB/SmolLM2-135M-Instruct/resolve/main/tokenizer.json",
    )
];

// SmolLM2-1.7B model files (medium size)
const SMOLLM_1_7B_FILES: [(&str, &str); 2] = [
    (
        "smollm_1_7b_model.safetensors",
        "https://huggingface.co/HuggingFaceTB/SmolLM2-1.7B-Instruct/resolve/main/model.safetensors",
    ),
    (
        "smollm_1_7b_tokenizer.json",
        "https://huggingface.co/HuggingFaceTB/SmolLM2-1.7B-Instruct/resolve/main/tokenizer.json",
    )
];

// Llama 3.2-3B model files (best quality for RAG)
const LLAMA32_3B_FILES: [(&str, &str); 3] = [
    (
        "llama32_3b_model.safetensors",
        "https://huggingface.co/meta-llama/Llama-3.2-3B-Instruct/resolve/main/model.safetensors",
    ),
    (
        "llama32_3b_tokenizer.json", 
        "https://huggingface.co/meta-llama/Llama-3.2-3B-Instruct/resolve/main/tokenizer.json",
    ),
    (
        "llama32_3b_config.json",
        "https://huggingface.co/meta-llama/Llama-3.2-3B-Instruct/resolve/main/config.json",
    )
];

pub async fn download_model(model_type: ModelType) -> Result<(), Box<dyn std::error::Error>> {
    if env_logger::try_init().is_err() {
        println!("Logger already initialized, continuing...");
    }
    
    let models_dir = PathBuf::from("models");
    fs::create_dir_all(&models_dir)?;
    
    let (files, model_name) = match model_type {
        ModelType::SmolLM2135M => {
            info!("Downloading SmolLM2-135M model files...");
            (&SMOLLM_135M_FILES[..], "SmolLM2-135M")
        }
        ModelType::SmolLM21_7B => {
            info!("Downloading SmolLM2-1.7B model files...");
            (&SMOLLM_1_7B_FILES[..], "SmolLM2-1.7B")
        }
        ModelType::Llama32_3B => {
            info!("Downloading Llama 3.2-3B model files...");
            (&LLAMA32_3B_FILES[..], "Llama 3.2-3B")
        }
    };
    
    for (filename, url) in files {
        let file_path = models_dir.join(filename);
        
        if file_path.exists() {
            info!("File {} already exists, skipping download", filename);
            continue;
        }
        
        info!("Downloading {} from {}", filename, url);
        download_with_progress(url, &file_path).await?;
        info!("Successfully downloaded {}", filename);
    }
    
    info!("{} model files downloaded successfully!", model_name);
    Ok(())
}

async fn download_with_progress(url: &str, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let total_size = response.content_length().unwrap_or(0);
    
    info!("Downloading {} bytes to {}", total_size, file_path.display());
    
    let mut file = tokio::fs::File::create(file_path).await?;
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();
    let mut last_progress_print = 0u64;
    
    use futures::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        
        if total_size > 0 {
            let progress = (downloaded as f64 / total_size as f64) * 100.0;
            // Print progress every 10MB or at completion
            if downloaded - last_progress_print >= 10 * 1024 * 1024 || downloaded == total_size {
                info!("Progress: {:.1}% ({}/{} MB)", 
                      progress, 
                      downloaded / (1024 * 1024), 
                      total_size / (1024 * 1024));
                last_progress_print = downloaded;
            }
        }
    }
    
    file.flush().await?;
    Ok(())
}
