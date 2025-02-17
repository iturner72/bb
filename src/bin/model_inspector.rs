use std::path::PathBuf;

#[cfg(feature = "ssr")]
async fn inspect_model() -> Result<(), Box<dyn std::error::Error>> {
    use memmap2::Mmap;
    use safetensors::SafeTensors;
    use std::fs::File;

    let models_dir = PathBuf::from("models");
    let model_path = models_dir.join("model.safetensors");

    log::info!("Opening model file: {}", model_path.display());
    let file = File::open(&model_path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let tensors = SafeTensors::deserialize(&mmap)?;

    log::info!("Model structure:");
    log::info!("----------------");
    
    let mut total_size = 0;
    for name in tensors.names() {
        let tensor = tensors.tensor(name)?;
        let size = tensor.data().len();
        total_size += size;
        
        log::info!(
            "{}: shape={:?}, dtype={:?}, size={:?} bytes",
            name,
            tensor.shape(),
            tensor.dtype(),
            size
        );
    }
    
    log::info!("----------------");
    log::info!("Total size: {} bytes", total_size);

    Ok(())
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use env_logger::Env;
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    
    match inspect_model().await {
        Ok(_) => log::info!("Model inspection completed successfully"),
        Err(e) => log::error!("Failed to inspect model: {}", e),
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {
    eprintln!("This binary requires the 'ssr' feature");
    std::process::exit(1);
}
