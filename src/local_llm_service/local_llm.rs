//! Local LLM Service using Candle with SmolLM2
//! Optimized for high TPS CPU inference

#[cfg(feature = "ssr")]
pub mod local_llm {
    use candle_core::{DType, Device, Tensor};
    use candle_nn::VarBuilder;
    use candle_transformers::models::llama::{Llama, Config as LlamaConfig, Cache, LlamaEosToks};
    use candle_transformers::generation::LogitsProcessor;
    use tokenizers::Tokenizer;
    use std::sync::OnceLock;
    use thiserror::Error;
    use log::{debug, info, warn, error};
    use std::path::PathBuf;
    use axum::response::sse::Event;
    use std::convert::Infallible;
    use tokio::sync::mpsc;
    use anyhow::Result;

    use crate::rag_service::rag::rag::{RagResponse, Citation};

    static LOCAL_LLM_SERVICE: OnceLock<LocalLLMService> = OnceLock::new();

    #[derive(Error, Debug)]
    pub enum LocalLLMError {
        #[error("Model error: {0}")]
        ModelError(#[from] candle_core::Error),

        #[error("Tokenizer error: {0}")]
        TokenizerError(#[from] tokenizers::Error),

        #[error("Initialization error: {0}")]
        InitError(String),

        #[error("Service not initialized")]
        NotInitialized,

        #[error("Generation error: {0}")]
        GenerationError(String),
    }

    #[derive(Debug, Clone)]
    pub struct GenerationConfig {
        pub max_new_tokens: usize,
        pub temperature: f32,
        pub top_p: f32,
        pub repetition_penalty: f32,
        pub seed: u64,
    }

    impl Default for GenerationConfig {
        fn default() -> Self {
            Self {
                max_new_tokens: 10,
                temperature: 0.3,
                top_p: 0.95,
                repetition_penalty: 1.2,
                seed: 42,
            }
        }
    }

    pub struct LocalLLMService {
        model: Llama,
        tokenizer: Tokenizer,
        device: Device,
        config: LlamaConfig,
    }

    impl LocalLLMService {
        pub fn new() -> Result<Self, LocalLLMError> {
            info!("Initializing LocalLLMService with SmolLM2-135M");

            let device = Device::Cpu;
            info!("Using device: {:?}", device);

            // Load tokenizer
            info!("Loading tokenizer");
            let tokenizer_path = PathBuf::from("models/smollm_tokenizer.json");
            let tokenizer = Tokenizer::from_file(&tokenizer_path)
                .map_err(LocalLLMError::TokenizerError)?;

            // SmolLM2-135M configuration (using actual Candle API)
            let config = LlamaConfig {
                hidden_size: 576,
                intermediate_size: 1536,
                vocab_size: 49152,
                num_hidden_layers: 30,
                num_attention_heads: 9,
                num_key_value_heads: 3,
                use_flash_attn: false,
                rms_norm_eps: 1e-5,
                rope_theta: 10000.0,
                bos_token_id: Some(1),
                eos_token_id: Some(LlamaEosToks::Single(2)),
                max_position_embeddings: 2048,
                rope_scaling: None,
                tie_word_embeddings: true,
            };

            // Load model weights
            info!("Loading model weights");
            unsafe {
                let vb = VarBuilder::from_mmaped_safetensors(
                    &["models/smollm_model.safetensors"],
                    DType::F32,
                    &device,
                )
                .map_err(|e| LocalLLMError::InitError(e.to_string()))?;

                let model = Llama::load(vb, &config)
                    .map_err(LocalLLMError::ModelError)?;

                Ok(Self {
                    model,
                    tokenizer,
                    device,
                    config,
                })
            }
        }

        pub fn get_instance() -> Result<&'static Self, LocalLLMError> {
            LOCAL_LLM_SERVICE
                .get()
                .ok_or(LocalLLMError::NotInitialized)
        }

        pub fn init() -> Result<(), LocalLLMError> {
            if LOCAL_LLM_SERVICE.get().is_none() {
                let service = Self::new()?;
                LOCAL_LLM_SERVICE.set(service).map_err(|_| {
                    LocalLLMError::InitError("Failed to initialize service".to_string())
                })?;
            }
            Ok(())
        }

        pub async fn generate_streaming_response(
            &self,
            prompt: String,
            tx: mpsc::Sender<Result<Event, Infallible>>,
            config: GenerationConfig,
        ) -> Result<(), LocalLLMError> {
            info!("Starting streaming generation for prompt length: {}", prompt.len());

            // Tokenize input
            debug!("📝 Tokenizing input...");
            let encoding = self.tokenizer
                .encode(prompt, true)
                .map_err(LocalLLMError::TokenizerError)?;
            
            let input_ids = encoding.get_ids();
            let max_length = 64; // Even smaller
            let input_ids = if input_ids.len() > max_length {
                warn!("Input too long ({}), truncating to {}", input_ids.len(), max_length);
                &input_ids[..max_length]
            } else {
                input_ids
            };
    
            debug!("🔢 Creating input tensor with {} tokens...", input_ids.len());
            let input_tensor = Tensor::new(input_ids, &self.device)?
                .to_dtype(DType::U32)?
                .unsqueeze(0)?;

            debug!("Input tokenized: {} tokens", input_ids.len());

            debug!("🧠 Initializing cache...");
            let mut cache = Cache::new(true, DType::F32, &self.config, &self.device)?;

            // Initialize generation
            debug!("🎯 Initializing logits processor...");
            let mut logits_processor = LogitsProcessor::new(
                config.seed,
                Some(config.temperature as f64),
                Some(config.top_p as f64),
            );

            let mut generated_tokens = Vec::new();
            let mut current_tensor = input_tensor;

            debug!("✅ Setup complete, starting generation loop...");

            // Generation loop
            for step in 0..config.max_new_tokens {
                debug!("🔄 Step {}: Starting forward pass with tensor shape: {:?}", step, current_tensor.dims());
                let start_time = std::time::Instant::now();
                // Forward pass
                debug!("🧠 Calling model.forward...");
                let logits = self.model.forward(&current_tensor, 0, &mut cache)?;
                let forward_time = start_time.elapsed();
                debug!("⚡ Forward pass completed in {:?}", forward_time);
                let logits = logits.squeeze(0)?.to_dtype(DType::F32)?;
                
                // Get next token
                let next_token = logits_processor.sample(&logits)?;
                let elapsed = start_time.elapsed();
                debug!("Generated token {} in {:?}: {}", step, elapsed, next_token);
                generated_tokens.push(next_token);

                // Check for EOS token
                if let Some(LlamaEosToks::Single(eos_id)) = &self.config.eos_token_id {
                    if next_token == *eos_id {
                        info!("Generated EOS token, stopping generation");
                        break;
                    }
                }

                // Decode the new token
                let new_text = match self.tokenizer.decode(&[next_token], false) {
                    Ok(text) => text,
                    Err(e) => {
                        warn!("Failed to decode token {}: {}", next_token, e);
                        continue;
                    }
                };

                // Send the new token via SSE
                let response = RagResponse {
                    message_type: "content".to_string(),
                    content: Some(new_text),
                    citations: None,
                };

                let json = serde_json::to_string(&response)
                    .map_err(|e| LocalLLMError::GenerationError(e.to_string()))?;
                
                if let Err(_) = tx.send(Ok(Event::default().data(json))).await {
                    info!("Client disconnected, stopping generation");
                    break;
                }

                // Prepare next input (just the new token)
                current_tensor = Tensor::new(&[next_token], &self.device)?
                    .to_dtype(DType::U32)?
                    .unsqueeze(0)?;

                // Small delay for better streaming experience
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

                if step % 20 == 0 {
                    debug!("Generated {} tokens", step + 1);
                }
            }

            info!("Generation completed. Total tokens: {}", generated_tokens.len());
            Ok(())
        }

        pub fn format_chat_prompt(&self, system: &str, user: &str, context: &str) -> String {
            // SmolLM2 uses this format
            format!(
                "<|im_start|>system\n{}\n\nContext:\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
                system, context, user
            )
        }
    }

    pub struct LocalRagService {
        llm_service: &'static LocalLLMService,
    }

    impl LocalRagService {
        pub fn new() -> Result<Self, LocalLLMError> {
            LocalLLMService::init()?;
            let llm_service = LocalLLMService::get_instance()?;
            Ok(Self { llm_service })
        }

        pub async fn process_query(
            &self,
            query: String,
            context: String,
            _citations: Vec<Citation>, // Prefixed with underscore to silence warning
            tx: mpsc::Sender<Result<Event, Infallible>>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            info!("Processing local RAG query: {}", query);

            // Send status update
            self.send_response(&tx, RagResponse {
                message_type: "status".to_string(),
                content: Some("Generating response with local SmolLM...".to_string()),
                citations: None,
            }).await?;

            // Format prompt for the model
            let system_prompt = "You are a helpful assistant that answers questions about blog posts. \
                Use the provided context to answer the user's question concisely. Reference specific posts when relevant.";

            let formatted_prompt = self.llm_service.format_chat_prompt(
                system_prompt,
                &query,
                &context,
            );

            // Generate response
            let config = GenerationConfig {
                max_new_tokens: 256, // Shorter for faster response
                temperature: 0.8,
                ..Default::default()
            };
            
            self.llm_service.generate_streaming_response(
                formatted_prompt,
                tx.clone(),
                config,
            ).await?;

            // Send completion signal
            self.send_response(&tx, RagResponse {
                message_type: "done".to_string(),
                content: None,
                citations: None,
            }).await?;

            Ok(())
        }

        async fn send_response(
            &self,
            tx: &mpsc::Sender<Result<Event, Infallible>>,
            response: RagResponse,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let json = serde_json::to_string(&response)?;
            tx.send(Ok(Event::default().data(json))).await
                .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;
            Ok(())
        }
    }

    // Test function for local LLM
    pub async fn test_local_llm() -> Result<(), Box<dyn std::error::Error>> {
        info!("Testing local LLM service");
        
        LocalLLMService::init()?;
        let service = LocalLLMService::get_instance()?;

        let test_prompt = service.format_chat_prompt(
            "You are a helpful assistant.",
            "What is cryptocurrency?",
            "Cryptocurrency is a digital currency secured by cryptography.",
        );

        let (tx, mut rx) = mpsc::channel(100);
        let config = GenerationConfig {
            max_new_tokens: 50, // Short test
            ..Default::default()
        };

        // Start generation in background
        let generation_handle = tokio::spawn(async move {
            service.generate_streaming_response(test_prompt, tx, config).await
        });

        // Collect response - fix the event data handling
        let mut full_response = String::new();
        while let Some(event_result) = rx.recv().await {
            match event_result {
                Ok(_event) => {
                    // For testing, we'll just collect tokens as they come
                    // In real usage, the SSE stream handles this properly
                    full_response.push_str(".");
                    print!(".");
                }
                Err(e) => {
                    // Handle the error case if needed
                    eprintln!("Error receiving event: {:?}", e);
                }
            }
        }

        generation_handle.await??;
        info!("\nGeneration test completed. Tokens generated: {}", full_response.len());
        Ok(())
    }
}
