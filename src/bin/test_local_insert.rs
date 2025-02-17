use dotenv::dotenv;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    log::info!("Starting local embedding test...");
    match bb::embeddings_service::embeddings_local::test_single_local_embedding().await {
        Ok(_) => log::info!("Test completed successfully"),
        Err(e) => log::error!("Test failed: {}", e),
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {
    eprintln!("This binary requires the 'ssr' feature");
    std::process::exit(1);
}
