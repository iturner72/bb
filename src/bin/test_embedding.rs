use dotenv::dotenv;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    log::info!("Starting embedding test...");

    match bb::embedding_service::embeddings::test_single_embedding().await {
        Ok(_) => log::info!("Test completed successfully!"),
        Err(e) => log::error!("Test failed: {}", e),
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {
    eprintln!("This binary requires the 'ssr' feature");
    std::process::exit(1);
}
