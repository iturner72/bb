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
    use crate::local_llm_service::download_llm_models::ModelType;

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

    impl GenerationConfig {
        // Default configs optimized for each model
        pub fn for_model(model_type: &ModelType) -> Self {
            match model_type {
                ModelType::SmolLM2135M => Self {
                    max_new_tokens: 100,        // Smaller for faster response
                    temperature: 0.8,           // Higher creativity for small model
                    top_p: 0.9,
                    repetition_penalty: 1.2,    // Higher to reduce repetition
                    seed: 42,
                },
                ModelType::SmolLM21_7B => Self {
                    max_new_tokens: 150,        // Medium length
                    temperature: 0.7,           // Balanced
                    top_p: 0.9,
                    repetition_penalty: 1.1,
                    seed: 42,
                },
                ModelType::Llama32_3B => Self {
                    max_new_tokens: 256,        // Longer for better quality
                    temperature: 0.6,           // Lower for more focused responses
                    top_p: 0.9,
                    repetition_penalty: 1.05,   // Lower penalty, model is better
                    seed: 42,
                },
            }
        }
    }

    impl Default for GenerationConfig {
        fn default() -> Self {
            Self::for_model(&ModelType::SmolLM2135M)
        }
    }

    #[derive(Debug, Clone)]
    pub struct ModelConfig {
        pub model_type: ModelType,
        pub model_path: String,
        pub tokenizer_path: String,
        pub config_path: Option<String>,
        pub llama_config: LlamaConfig,
    }

    impl ModelConfig {
        pub fn new(model_type: ModelType) -> Self {
            match model_type {
                ModelType::SmolLM2135M => Self {
                    model_type: model_type.clone(),
                    model_path: "models/smollm_135m_model.safetensors".to_string(),
                    tokenizer_path: "models/smollm_135m_tokenizer.json".to_string(),
                    config_path: None,
                    llama_config: LlamaConfig {
                        hidden_size: 576,
                        intermediate_size: 1536,
                        vocab_size: 49152,
                        num_hidden_layers: 24,
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
                    },
                },
                ModelType::SmolLM21_7B => Self {
                    model_type: model_type.clone(),
                    model_path: "models/smollm_1_7b_model.safetensors".to_string(),
                    tokenizer_path: "models/smollm_1_7b_tokenizer.json".to_string(),
                    config_path: None,
                    llama_config: LlamaConfig {
                        hidden_size: 2048,
                        intermediate_size: 8192,
                        vocab_size: 49152,
                        num_hidden_layers: 24,
                        num_attention_heads: 32,
                        num_key_value_heads: 32,
                        use_flash_attn: false,
                        rms_norm_eps: 1e-5,
                        rope_theta: 10000.0,
                        bos_token_id: Some(1),
                        eos_token_id: Some(LlamaEosToks::Single(2)),
                        max_position_embeddings: 2048,
                        rope_scaling: None,
                        tie_word_embeddings: true,
                    },
                },
                ModelType::Llama32_3B => Self {
                    model_type: model_type.clone(),
                    model_path: "models/llama32_3b_model.safetensors".to_string(),
                    tokenizer_path: "models/llama32_3b_tokenizer.json".to_string(),
                    config_path: Some("models/llama32_3b_config.json".to_string()),
                    llama_config: LlamaConfig {
                        hidden_size: 3072,
                        intermediate_size: 8192,
                        vocab_size: 128256,
                        num_hidden_layers: 28,
                        num_attention_heads: 24,
                        num_key_value_heads: 8,
                        use_flash_attn: false,
                        rms_norm_eps: 1e-5,
                        rope_theta: 500000.0,  // Higher for Llama 3.2
                        bos_token_id: Some(128000),
                        eos_token_id: Some(LlamaEosToks::Single(128001)),
                        max_position_embeddings: 131072,  // Much larger context
                        rope_scaling: None,
                        tie_word_embeddings: false,  // Llama 3.2 doesn't tie embeddings
                    },
                },
            }
        }
        
        pub fn get_model_name(&self) -> &'static str {
            match self.model_type {
                ModelType::SmolLM2135M => "SmolLM2-135M",
                ModelType::SmolLM21_7B => "SmolLM2-1.7B", 
                ModelType::Llama32_3B => "Llama 3.2-3B",
            }
        }
    }

    pub struct LocalLLMService {
        model: Llama,
        tokenizer: Tokenizer,
        device: Device,
        config: LlamaConfig,
        model_config: ModelConfig,
    }

    impl LocalLLMService {
        pub fn new(model_type: ModelType) -> Result<Self, LocalLLMError> {
            let model_config = ModelConfig::new(model_type);
            
            info!("Initializing LocalLLMService with {}", model_config.get_model_name());
            
            let device = Device::Cpu;
            info!("Using device: {:?}", device);
            
            // Load tokenizer
            info!("Loading tokenizer from: {}", model_config.tokenizer_path);
            let tokenizer_path = PathBuf::from(&model_config.tokenizer_path);
            let tokenizer = Tokenizer::from_file(&tokenizer_path)
                .map_err(LocalLLMError::TokenizerError)?;
            
            // Load model weights
            info!("Loading model weights from: {}", model_config.model_path);
            unsafe {
                let vb = VarBuilder::from_mmaped_safetensors(
                    &[&model_config.model_path],
                    DType::F32,
                    &device,
                )
                .map_err(|e| LocalLLMError::InitError(e.to_string()))?;
                
                let model = Llama::load(vb, &model_config.llama_config)
                    .map_err(LocalLLMError::ModelError)?;
                
                Ok(Self {
                    model,
                    tokenizer,
                    device,
                    config: model_config.llama_config.clone(),
                    model_config,
                })
            }
        }
        
        pub fn get_instance() -> Result<&'static Self, LocalLLMError> {
            LOCAL_LLM_SERVICE
                .get()
                .ok_or(LocalLLMError::NotInitialized)
        }
        
        pub fn init(model_type: ModelType) -> Result<(), LocalLLMError> {
            if LOCAL_LLM_SERVICE.get().is_none() {
                let service = Self::new(model_type)?;
                LOCAL_LLM_SERVICE.set(service).map_err(|_| {
                    LocalLLMError::InitError("Failed to initialize service".to_string())
                })?;
            }
            Ok(())
        }
        
        pub fn get_model_type(&self) -> &ModelType {
            &self.model_config.model_type
        }
        
        pub fn format_chat_prompt(&self, system: &str, user: &str, context: &str) -> String {
            match self.model_config.model_type {
                ModelType::SmolLM2135M | ModelType::SmolLM21_7B => {
                    // SmolLM2 uses ChatML format
                    format!(
                        "<|im_start|>system\n{}\n\nContext:\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
                        system, context, user
                    )
                },
                ModelType::Llama32_3B => {
                    // Llama 3.2 uses different format
                    format!(
                        "<|begin_of_text|><|start_header_id|>system<|end_header_id|>\n{}\n\nContext:\n{}<|eot_id|><|start_header_id|>user<|end_header_id|>\n{}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n",
                        system, context, user
                    )
                }
            }
        }
        
        pub fn get_optimized_generation_config(&self) -> GenerationConfig {
            GenerationConfig::for_model(&self.model_config.model_type)
        }

        pub async fn generate_streaming_response(
            &self,
            prompt: String,
            tx: mpsc::Sender<Result<Event, Infallible>>,
            config: GenerationConfig,
        ) -> Result<(), LocalLLMError> {
            info!("Starting streaming generation for {} (prompt length: {})", 
                  self.model_config.get_model_name(), 
                  prompt.len());
    
            // Tokenize input
            debug!("📝 Tokenizing input...");
            let encoding = self.tokenizer
                .encode(prompt, true)
                .map_err(LocalLLMError::TokenizerError)?;
            
            let input_ids = encoding.get_ids();
            
            // Dynamic context limits based on model
            let max_input_length = match self.model_config.model_type {
                ModelType::SmolLM2135M => 512,      // Smaller context for tiny model
                ModelType::SmolLM21_7B => 1024,     // Medium context
                ModelType::Llama32_3B => 4096,      // Large context for best model
            };
            
            let input_ids = if input_ids.len() > max_input_length {
                warn!("Input too long ({}), truncating to {}", input_ids.len(), max_input_length);
                &input_ids[..max_input_length]
            } else {
                input_ids
            };
    
            debug!("🔢 Creating input tensor with {} tokens...", input_ids.len());
            let input_tensor = Tensor::new(input_ids, &self.device)?
                .to_dtype(DType::U32)?
                .unsqueeze(0)?;
    
            debug!("🧠 Initializing cache...");
            let mut cache = Cache::new(true, DType::F32, &self.config, &self.device)?;
    
            // Initialize generation with model-specific settings
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
                debug!("🔄 Step {}: Starting forward pass", step);
                let start_time = std::time::Instant::now();
                
                // Forward pass
                let logits = self.model.forward(&current_tensor, 0, &mut cache)?;
                let forward_time = start_time.elapsed();
                debug!("⚡ Forward pass completed in {:?}", forward_time);
                
                let logits = logits.squeeze(0)?.to_dtype(DType::F32)?;
                
                // Apply repetition penalty if needed
                let logits = if config.repetition_penalty != 1.0 && !generated_tokens.is_empty() {
                    self.apply_repetition_penalty(&logits, &generated_tokens, config.repetition_penalty)?
                } else {
                    logits
                };
                
                // Get next token
                let next_token = logits_processor.sample(&logits)?;
                let elapsed = start_time.elapsed();
                debug!("Generated token {} in {:?}: {}", step, elapsed, next_token);
                generated_tokens.push(next_token);
    
                // Check for EOS token - handle different models
                let should_stop = match &self.config.eos_token_id {
                    Some(LlamaEosToks::Single(eos_id)) => next_token == *eos_id,
                    Some(LlamaEosToks::Multiple(eos_ids)) => eos_ids.contains(&next_token),
                    None => false,
                };
                
                if should_stop {
                    info!("Generated EOS token, stopping generation");
                    break;
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
    
                // Model-specific delays for better streaming
                let delay_ms = match self.model_config.model_type {
                    ModelType::SmolLM2135M => 5,    // Faster streaming for small model
                    ModelType::SmolLM21_7B => 10,   // Medium delay
                    ModelType::Llama32_3B => 20,    // Slower for larger model
                };
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    
                if step % 20 == 0 {
                    debug!("Generated {} tokens", step + 1);
                }
            }
    
            info!("Generation completed for {}. Total tokens: {}", 
                  self.model_config.get_model_name(),
                  generated_tokens.len());
            Ok(())
        }

        fn apply_repetition_penalty(
            &self,
            logits: &Tensor,
            generated_tokens: &[u32],
            penalty: f32,
        ) -> Result<Tensor, LocalLLMError> {
            if penalty == 1.0 {
                return Ok(logits.clone());
            }
    
            let mut logits_vec = logits.to_vec1::<f32>()?;
            
            // Apply penalty to tokens that have been generated
            for &token in generated_tokens.iter().rev().take(50) { // Look at last 50 tokens
                if (token as usize) < logits_vec.len() {
                    let current_score = logits_vec[token as usize];
                    logits_vec[token as usize] = if current_score > 0.0 {
                        current_score / penalty
                    } else {
                        current_score * penalty
                    };
                }
            }
    
            Tensor::from_vec(logits_vec, logits.shape(), &self.device)
                .map_err(LocalLLMError::ModelError)
        }
    }

    pub struct LocalRagService {
        llm_service: &'static LocalLLMService,
    }

    impl LocalRagService {
        pub fn new(model_type: ModelType) -> Result<Self, LocalLLMError> {
            LocalLLMService::init(model_type)?;
            let llm_service = LocalLLMService::get_instance()?;
            Ok(Self { llm_service })
        }
    
        pub async fn process_query(
            &self,
            query: String,
            context: String,
            _citations: Vec<Citation>,
            tx: mpsc::Sender<Result<Event, Infallible>>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            info!("Processing local RAG query with {}: {}", 
                  self.llm_service.model_config.get_model_name(), 
                  query);
    
            // Send status update
            self.send_response(&tx, RagResponse {
                message_type: "status".to_string(),
                content: Some(format!("Generating response with {}...", 
                                    self.llm_service.model_config.get_model_name())),
                citations: None,
            }).await?;
    
            // Format prompt for the specific model
            let system_prompt = match self.llm_service.get_model_type() {
                ModelType::SmolLM2135M => {
                    "You are a helpful assistant. Answer briefly using the context provided."
                },
                ModelType::SmolLM21_7B => {
                    "You are a helpful assistant that answers questions about blog posts. \
                     Use the provided context to answer the user's question concisely."
                },
                ModelType::Llama32_3B => {
                    "You are an expert assistant that provides detailed, accurate answers based on the given context. \
                     Reference specific information from the context and explain your reasoning when relevant."
                }
            };
    
            let formatted_prompt = self.llm_service.format_chat_prompt(
                system_prompt,
                &query,
                &context,
            );
    
            // Use model-optimized generation config
            let config = self.llm_service.get_optimized_generation_config();
            
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
    
    // Updated test function
    pub async fn test_local_llm(model_type: ModelType) -> Result<(), Box<dyn std::error::Error>> {
        info!("Testing local LLM service with {:?}", model_type);
        
        LocalLLMService::init(model_type)?;
        let service = LocalLLMService::get_instance()?;
    
        let test_prompt = service.format_chat_prompt(
            "You are a helpful assistant.",
            "What is cryptocurrency?",
            "Cryptocurrency is a digital currency secured by cryptography.",
        );
    
        let (tx, mut rx) = mpsc::channel(100);
        let config = service.get_optimized_generation_config();
    
        // Start generation in background
        let generation_handle = tokio::spawn(async move {
            service.generate_streaming_response(test_prompt, tx, config).await
        });
    
        // Collect response
        let mut token_count = 0;
        while let Some(event_result) = rx.recv().await {
            match event_result {
                Ok(_event) => {
                    token_count += 1;
                    print!(".");
                    if token_count % 10 == 0 {
                        print!(" ");
                    }
                }
                Err(e) => {
                    eprintln!("Error receiving event: {:?}", e);
                }
            }
        }
    
        generation_handle.await??;
        info!("\nGeneration test completed with {}. Tokens generated: {}", 
              service.model_config.get_model_name(), 
              token_count);
        Ok(())
    }
}
