//! src/bin/download_smollm.rs
//! Download SmolLM2-135M model for local LLM inference

use dotenv::dotenv;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    dotenv().ok();
    
    // Initialize logger only if not already initialized
    if env_logger::try_init().is_err() {
        println!("Logger already initialized, continuing...");
    }

    log::info!("Starting SmolLM2-135M model download...");
    
    match bb::local_llm_service::download_smollm::download_model(
        bb::local_llm_service::download_smollm::ModelType::SmolLM2
    ).await {
        Ok(_) => {
            log::info!("SmolLM2-135M model downloaded successfully!");
            log::info!("You can now use the local LLM service.");
        }
        Err(e) => {
            log::error!("Failed to download model: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {
    eprintln!("This binary requires the 'ssr' feature");
    std::process::exit(1);
}

#[cfg(not(feature = "ssr"))]
fn main() {
    eprintln!("This binary requires the 'ssr' feature");
    std::process::exit(1);
}
