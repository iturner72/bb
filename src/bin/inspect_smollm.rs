//! src/bin/inspect_smollm.rs
//! Inspect the SmolLM2 model structure to see what tensors are available

use std::path::PathBuf;

#[cfg(feature = "ssr")]
async fn inspect_smollm_model() -> Result<(), Box<dyn std::error::Error>> {
    use memmap2::Mmap;
    use safetensors::SafeTensors;
    use std::fs::File;

    let models_dir = PathBuf::from("models");
    let model_path = models_dir.join("smollm_model.safetensors");

    if !model_path.exists() {
        println!("Model file not found at: {}", model_path.display());
        println!("Please run: cargo run --bin download_smollm --features ssr");
        return Ok(());
    }

    println!("Opening model file: {}", model_path.display());
    let file = File::open(&model_path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let tensors = SafeTensors::deserialize(&mmap)?;

    println!("SmolLM2 Model structure:");
    println!("========================");
    
    let mut total_size = 0;
    let mut tensor_names: Vec<_> = tensors.names().into_iter().collect();
    tensor_names.sort();
    
    for name in tensor_names {
        let tensor = tensors.tensor(name)?;
        let size = tensor.data().len();
        total_size += size;
        
        println!(
            "{}: shape={:?}, dtype={:?}, size={} bytes",
            name,
            tensor.shape(),
            tensor.dtype(),
            size
        );
    }
    
    println!("========================");
    println!("Total tensors: {}", tensors.names().len());
    println!("Total size: {} bytes ({:.2} MB)", total_size, total_size as f64 / 1024.0 / 1024.0);

    // Look for specific patterns
    println!("\nTensor name patterns:");
    let names: Vec<_> = tensors.names().into_iter().collect();
    
    if names.iter().any(|n| n.contains("lm_head")) {
        println!("✓ Found lm_head tensors");
    } else {
        println!("✗ No lm_head tensors found");
        println!("  Available head-like tensors:");
        for name in names.iter().filter(|n| n.contains("head") || n.ends_with("weight")) {
            println!("    - {}", name);
        }
    }
    
    if names.iter().any(|n| n.contains("embed_tokens")) {
        println!("✓ Found embed_tokens");
    } else {
        println!("✗ No embed_tokens found");
        println!("  Available embedding-like tensors:");
        for name in names.iter().filter(|n| n.contains("embed") || n.contains("token")) {
            println!("    - {}", name);
        }
    }

    Ok(())
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    env_logger::init();
    
    match inspect_smollm_model().await {
        Ok(_) => println!("Model inspection completed successfully"),
        Err(e) => println!("Failed to inspect model: {}", e),
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {
    eprintln!("This binary requires the 'ssr' feature");
    std::process::exit(1);
}
