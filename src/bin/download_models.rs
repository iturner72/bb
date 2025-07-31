#[cfg(feature = "ssr")]
use log::{error, info};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

const MODEL_FILES: [(&str, &str, &str); 2] = [
    (
        "model.safetensors",
        "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/model.safetensors",
        "53aa51172d142c89d9012cce15ae4d6cc0ca6895895114379cacb4fab128d9db",
    ),
    (
        "tokenizer.json",
        "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json",
        "be50c3628f2bf5bb5e3a7f17b1f74611b2561a3a27eeab05e5aa30f411572037",
    )
];

async fn download_and_verify() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let models_dir = PathBuf::from("ml_models");
    fs::create_dir_all(&models_dir)?;

    for (filename, url, expected_hash) in MODEL_FILES {
        let file_path = models_dir.join(filename);
        
        if file_path.exists() {
            info!("Checking existing file: {}", filename);
            let content = fs::read(&file_path)?;
            let mut hasher = Sha256::new();
            hasher.update(&content);
            let hash = format!("{:x}", hasher.finalize());
            
            if hash == expected_hash {
                info!("File {} already exists and hash matches", filename);
                continue;
            }
            info!("File {} exists but hash doesn't match, re-downloading", filename);
        }

        info!("Downloading {} to {}", url, file_path.display());
        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        fs::write(&file_path, &bytes)?;
        info!("Successfully downloaded to {}", file_path.display());
        
        let content = fs::read(&file_path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = format!("{:x}", hasher.finalize());
        
        if hash != expected_hash {
            error!("Hash verification failed for {}", filename);
            fs::remove_file(&file_path)?;
            return Err(format!("Hash verification failed for {}", filename).into());
        }
        
        info!("Successfully verified {}", filename);
    }

    info!("All model files downloaded and verified successfully!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    download_and_verify().await
}
