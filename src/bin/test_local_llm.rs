//! src/bin/test_local_llm.rs
//! Test the local LLM service

use dotenv::dotenv;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    dotenv().ok();
    
    // Initialize logger only if not already initialized
    if env_logger::try_init().is_err() {
        println!("Logger already initialized, continuing...");
    }

    log::info!("Testing local LLM service...");
    
    match bb::local_llm_service::local_llm::test_local_llm().await {
        Ok(_) => {
            log::info!("Local LLM test completed successfully!");
        }
        Err(e) => {
            log::error!("Local LLM test failed: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {
    eprintln!("This binary requires the 'ssr' feature");
    std::process::exit(1);
}
