//! src/bin/download_llm_models.rs
//! Download various LLM models for local inference
use dotenv::dotenv;
use std::env;

fn print_usage() {
    println!("Usage: cargo run --bin download_llm_models --features ssr -- --model <MODEL>");
    println!();
    println!("Available models:");
    println!("  smol-lm2-135m    SmolLM2-135M (smallest, fastest, ~270MB)");
    println!("  smol-lm2-1-7b    SmolLM2-1.7B (medium size, ~3.4GB)");
    println!("  llama32-3b       Llama 3.2-3B (best quality for RAG, ~6GB)");
    println!();
    println!("Examples:");
    println!("  cargo run --bin download_llm_models --features ssr -- --model smol-lm2-135m");
    println!("  cargo run --bin download_llm_models --features ssr -- --model llama32-3b");
}

fn parse_model_arg(model_str: &str) -> Option<bb::local_llm_service::download_llm_models::ModelType> {
    match model_str {
        "smol-lm2-135m" => Some(bb::local_llm_service::download_llm_models::ModelType::SmolLM2135M),
        "smol-lm2-1-7b" => Some(bb::local_llm_service::download_llm_models::ModelType::SmolLM21_7B),
        "llama32-3b" => Some(bb::local_llm_service::download_llm_models::ModelType::Llama32_3B),
        _ => None,
    }
}

fn get_model_name(model_type: &bb::local_llm_service::download_llm_models::ModelType) -> &'static str {
    match model_type {
        bb::local_llm_service::download_llm_models::ModelType::SmolLM2135M => "SmolLM2-135M",
        bb::local_llm_service::download_llm_models::ModelType::SmolLM21_7B => "SmolLM2-1.7B",
        bb::local_llm_service::download_llm_models::ModelType::Llama32_3B => "Llama 3.2-3B",
    }
}

// Always compile main function, but only use download functionality with ssr feature
#[tokio::main]
async fn main() {
    #[cfg(not(feature = "ssr"))]
    {
        eprintln!("This binary requires the 'ssr' feature to be enabled.");
        eprintln!("Run with: cargo run --bin download_llm_models --features ssr -- --model <MODEL>");
        std::process::exit(1);
    }

    #[cfg(feature = "ssr")]
    {
        dotenv().ok();
        
        // Initialize logger only if not already initialized
        if env_logger::try_init().is_err() {
            println!("Logger already initialized, continuing...");
        }
        
        let args: Vec<String> = env::args().collect();
        
        // Parse command line arguments
        let model_type = if args.len() >= 3 && args[1] == "--model" {
            match parse_model_arg(&args[2]) {
                Some(model) => model,
                None => {
                    eprintln!("Error: Unknown model '{}'", args[2]);
                    print_usage();
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("Error: Missing or invalid arguments");
            print_usage();
            std::process::exit(1);
        };
        
        let model_name = get_model_name(&model_type);
        log::info!("Starting {} model download...", model_name);
        
        match bb::local_llm_service::download_llm_models::download_model(model_type).await {
            Ok(_) => {
                log::info!("{} model downloaded successfully!", model_name);
                log::info!("You can now use the local LLM service.");
            }
            Err(e) => {
                log::error!("Failed to download model: {}", e);
                std::process::exit(1);
            }
        }
    }
}
