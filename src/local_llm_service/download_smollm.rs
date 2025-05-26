//! src/local_llm_service/download_smollm.rs
//! SmolLM2-135M model downloader
//! Downloads optimized SmolLM2-135M model files for local inference

use log::{error, info};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

// SmolLM2-135M model files for CPU inference (much smaller and faster)
const SMOLLM_MODEL_FILES: [(&str, &str); 2] = [
    (
        "smollm_model.safetensors",
        "https://huggingface.co/HuggingFaceTB/SmolLM2-135M-Instruct/resolve/main/model.safetensors",
    ),
    (
        "smollm_tokenizer.json",
        "https://huggingface.co/HuggingFaceTB/SmolLM2-135M-Instruct/resolve/main/tokenizer.json",
    )
];

pub enum ModelType {
    SmolLM2,
}

pub async fn download_model(model_type: ModelType) -> Result<(), Box<dyn std::error::Error>> {
    if env_logger::try_init().is_err() {
        println!("Logger already initialized, continuing...");
    }

    let models_dir = PathBuf::from("models");
    fs::create_dir_all(&models_dir)?;

    let files = match model_type {
        ModelType::SmolLM2 => {
            info!("Downloading SmolLM2-135M model files...");
            &SMOLLM_MODEL_FILES[..]
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

    match model_type {
        ModelType::SmolLM2 => info!("SmolLM2-135M model files downloaded successfully!"),
    }
    
    Ok(())
}

async fn download_with_progress(url: &str, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let total_size = response.content_length().unwrap_or(0);
    
    info!("Downloading {} bytes to {}", total_size, file_path.display());
    
    let mut file = tokio::fs::File::create(file_path).await?;
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();
    
    use futures::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        
        if total_size > 0 {
            let progress = (downloaded as f64 / total_size as f64) * 100.0;
            if downloaded % (1024 * 1024) == 0 || downloaded == total_size { // Print every MB or at end
                info!("Progress: {:.1}% ({}/{})", progress, downloaded, total_size);
            }
        }
    }
    
    file.flush().await?;
    Ok(())
}
