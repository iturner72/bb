<picture>
    <source srcset="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_Solid_White.svg" media="(prefers-color-scheme: dark)">
    <img src="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_RGB.svg" alt="Leptos Logo">
</picture>

# Leptos Axum Starter Template

This is a template for use with the [Leptos](https://github.com/leptos-rs/leptos) web framework and the [cargo-leptos](https://github.com/akesson/cargo-leptos) tool using [Axum](https://github.com/tokio-rs/axum).

## Creating your template repo

If you don't have `cargo-leptos` installed you can install it with

```bash
cargo install cargo-leptos --locked
```

Then run
```bash
cargo leptos new --git leptos-rs/start-axum
```

to generate a new project template.

```bash
cd bb
```

to go to your newly created project.  
Feel free to explore the project structure, but the best place to start with your application code is in `src/app.rs`.  
Addtionally, Cargo.toml may need updating as new versions of the dependencies are released, especially if things are not working after a `cargo update`.

## Running your project

```bash
cargo leptos watch
```

## Installing Additional Tools

By default, `cargo-leptos` uses `nightly` Rust, `cargo-generate`, and `sass`. If you run into any trouble, you may need to install one or more of these tools.

1. `rustup toolchain install nightly --allow-downgrade` - make sure you have Rust nightly
2. `rustup target add wasm32-unknown-unknown` - add the ability to compile Rust to WebAssembly
3. `cargo install cargo-generate` - install `cargo-generate` binary (should be installed automatically in future)
4. `npm install -g sass` - install `dart-sass` (should be optional in future

## Compiling for Release
```bash
cargo leptos build --release
```

Will generate your server binary in target/server/release and your site package in target/site

## Testing Your Project
```bash
cargo leptos end-to-end
```

```bash
cargo leptos end-to-end --release
```

Cargo-leptos uses Playwright as the end-to-end test tool.  
Tests are located in end2end/tests directory.

## Executing a Server on a Remote Machine Without the Toolchain
After running a `cargo leptos build --release` the minimum files needed are:

1. The server binary located in `target/server/release`
2. The `site` directory and all files within located in `target/site`

Copy these files to your remote server. The directory structure should be:
```text
bb
site/
```
Set the following environment variables (updating for your project as needed):
```text
LEPTOS_OUTPUT_NAME="bb"
LEPTOS_SITE_ROOT="site"
LEPTOS_SITE_PKG_DIR="pkg"
LEPTOS_SITE_ADDR="127.0.0.1:3000"
LEPTOS_RELOAD_PORT="3001"
```
Finally, run the server binary.

## Licensing

This template itself is released under the Unlicense. You should replace the LICENSE for your own application with an appropriate license if you plan to release it publicly.

## Embeddings Help
helper embedding command to test single embedding (if there exists a post
without one)
```bash
RUST_LOG=debug cargo run --bin test_embedding --features ssr
```

## Local embedding model download
cargo run --bin download_models --features ssr

# Local LLM Model Downloads

This guide explains how to download and use different LLM models for local inference.

## Available Models

### SmolLM2-135M (Smallest, Fastest)
- **Size**: ~270MB  
- **Speed**: Very fast inference
- **Quality**: Basic responses, good for testing
- **Use case**: Development and testing

### SmolLM2-1.7B (Medium)
- **Size**: ~3.4GB
- **Speed**: Fast inference  
- **Quality**: Decent responses
- **Use case**: Lightweight production

### Llama 3.2-3B (Recommended for RAG)
- **Size**: ~6GB
- **Speed**: Good inference speed
- **Quality**: Excellent reasoning and instruction following
- **Use case**: Production RAG applications

## Download Commands

### Download Embedding Models
```bash
cargo run --bin download_models --features ssr
```

### Download LLM Models

#### Download SmolLM2-135M (smallest, for testing)
```bash
cargo run --bin download_llm_models --features ssr -- --model smol-lm2-135m
```

#### Download SmolLM2-1.7B (medium size)
```bash  
cargo run --bin download_llm_models --features ssr -- --model smol-lm2-1-7b
```

#### Download Llama 3.2-3B (recommended for production RAG)
```bash
cargo run --bin download_llm_models --features ssr -- --model llama32-3b
```

## Model Selection Guide

| Model | Size | RAM Usage | Speed | RAG Quality | Best For |
|-------|------|-----------|-------|-------------|----------|
| SmolLM2-135M | 270MB | ~1GB | Very Fast | Poor | Testing, Development |
| SmolLM2-1.7B | 3.4GB | ~5GB | Fast | Fair | Lightweight Apps |
| Llama 3.2-3B | 6GB | ~8GB | Good | Excellent | Production RAG |

## Hardware Requirements

- **Minimum RAM**: 8GB (for smallest model)
- **Recommended RAM**: 16GB+ (for Llama 3.2-3B)
- **Storage**: 1-10GB depending on model
- **CPU**: Modern multi-core processor recommended

## Usage After Download

After downloading, the models will be available in the `models/` directory. Update your local LLM service configuration to use the desired model files.

For Llama 3.2-3B, you'll need to update your service to use:
- `llama32_3b_model.safetensors`
- `llama32_3b_tokenizer.json` 
- `llama32_3b_config.json`

## Performance Tips

1. **Start with SmolLM2-135M** for initial testing
2. **Use Llama 3.2-3B** for production RAG applications
3. **Monitor memory usage** - models load entirely into RAM
4. **Consider quantization** for even smaller memory footprint

## Troubleshooting

- **Download fails**: Check internet connection and HuggingFace availability
- **Out of memory**: Try a smaller model or increase system RAM
- **Slow performance**: Ensure sufficient RAM and consider CPU optimization flags
