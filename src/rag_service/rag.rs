#[cfg(feature = "ssr")]
pub mod rag {
    use async_openai::{
        config::OpenAIConfig,
        types::{
            ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
            ChatCompletionRequestUserMessage, CreateChatCompletionRequest,
        },
        Client,
    };
    use axum::response::sse::Event;
    use futures::StreamExt;
    use log::{error, info, warn};
    use serde::{Deserialize, Serialize};
    use std::convert::Infallible;
    use tokio::sync::mpsc;
    use anyhow::Result;

    use crate::components::poasts::{semantic_search, Poast};
    use crate::components::search::SearchType;
    use crate::local_llm_service::local_llm::local_llm::{LocalRagService, LocalLLMError};
    use crate::local_llm_service::download_llm_models::ModelType;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct RagMessage {
        pub role: String,
        pub content: String,
        pub citations: Option<Vec<Citation>>,
        pub timestamp: String,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Citation {
        pub title: String,
        pub company: String,
        pub link: String,
        pub published_at: String,
        pub relevance_score: f32,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct RagResponse {
        pub message_type: String, // "content", "citation", "error", "done", "status"
        pub content: Option<String>,
        pub citations: Option<Vec<Citation>>,
    }

    #[derive(Debug, Clone)]
    pub enum LLMProvider {
        OpenAI,
        Local,
    }

    pub struct RagService {
        openai_client: Option<Client<OpenAIConfig>>,
        local_service: Option<LocalRagService>,
        provider: LLMProvider,
        model: String,
    }

    impl RagService {
        pub fn new_openai() -> Self {
            let client = Client::new();
            Self {
                openai_client: Some(client),
                local_service: None,
                provider: LLMProvider::OpenAI,
                model: "gpt-3.5-turbo".to_string(),
            }
        }

        pub fn new_local() -> Result<Self, LocalLLMError> {
            let local_service = LocalRagService::new(ModelType::Llama32_3B)?;
            Ok(Self {
                openai_client: None,
                local_service: Some(local_service),
                provider: LLMProvider::Local,
                model: "phi-3.5-mini".to_string(),
            })
        }

        pub async fn process_query(
            &self,
            query: String,
            search_type: SearchType,
            tx: mpsc::Sender<Result<Event, Infallible>>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            info!("Processing RAG query with {:?} LLM: {}", self.provider, query);

            // Step 1: Send initial status
            self.send_response(&tx, RagResponse {
                message_type: "status".to_string(),
                content: Some("Searching relevant posts...".to_string()),
                citations: None,
            }).await?;

            // Step 2: Get relevant posts using existing semantic search
            let relevant_posts = match semantic_search(query.clone(), search_type).await {
                Ok(posts) => posts,
                Err(e) => {
                    error!("Failed to perform semantic search: {}", e);
                    self.send_response(&tx, RagResponse {
                        message_type: "error".to_string(),
                        content: Some("Failed to search relevant posts".to_string()),
                        citations: None,
                    }).await?;
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Semantic search failed: {}", e))));
                }
            };

            info!("Found {} relevant posts", relevant_posts.len());

            // Step 3: Take top 5 and create citations
            let top_posts: Vec<Poast> = relevant_posts.into_iter().take(5).collect();
            
            if top_posts.is_empty() {
                self.send_response(&tx, RagResponse {
                    message_type: "error".to_string(),
                    content: Some("No relevant posts found for your query.".to_string()),
                    citations: None,
                }).await?;
                return Ok(());
            }

            let citations: Vec<Citation> = top_posts
                .iter()
                .enumerate()
                .map(|(i, post)| Citation {
                    title: post.title.clone(),
                    company: post.company.clone(),
                    link: post.link.clone(),
                    published_at: post.published_at.clone(),
                    relevance_score: 1.0 - (i as f32 * 0.1), // Simple decreasing score
                })
                .collect();

            // Step 4: Send citations
            self.send_response(&tx, RagResponse {
                message_type: "citations".to_string(),
                content: None,
                citations: Some(citations.clone()),
            }).await?;

            // Step 5: Create context from posts
            let context = self.create_context(&top_posts);
            
            // Step 6: Generate response based on provider
            match self.provider {
                LLMProvider::OpenAI => {
                    if let Some(ref client) = self.openai_client {
                        self.generate_openai_response(query, context, tx, client).await?;
                    } else {
                        return Err("OpenAI client not initialized".into());
                    }
                }
                LLMProvider::Local => {
                    if let Some(ref local_service) = self.local_service {
                        local_service.process_query(query, context, citations, tx).await?;
                    } else {
                        return Err("Local LLM service not initialized".into());
                    }
                }
            }

            Ok(())
        }

        fn create_context(&self, posts: &[Poast]) -> String {
            let mut context = String::new();
            context.push_str("Here are relevant blog posts to help answer the user's question:\n\n");

            for (i, post) in posts.iter().enumerate() {
                context.push_str(&format!("Post {}:\n", i + 1));
                context.push_str(&format!("Title: {}\n", post.title));
                context.push_str(&format!("Company: {}\n", post.company));
                context.push_str(&format!("Published: {}\n", post.published_at));
                
                if let Some(summary) = &post.summary {
                    context.push_str(&format!("Summary: {}\n", summary));
                }
                
                context.push_str(&format!("Link: {}\n\n", post.link));
            }

            context
        }

        async fn generate_openai_response(
            &self,
            query: String,
            context: String,
            tx: mpsc::Sender<Result<Event, Infallible>>,
            client: &Client<OpenAIConfig>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            // Send status update
            self.send_response(&tx, RagResponse {
                message_type: "status".to_string(),
                content: Some("Generating response with OpenAI...".to_string()),
                citations: None,
            }).await?;

            let system_message = ChatCompletionRequestSystemMessage {
                content: format!(
                    "You are a helpful assistant that answers questions about blog posts. \
                    Use the provided context to answer the user's question. If the context doesn't contain enough \
                    information to answer the question, say so clearly. Always reference specific posts when relevant by \
                    mentioning the company name and post title. Be concise but informative. Format your response in markdown.\n\n\
                    When referencing posts, use this format: **[Company Name - Post Title]**\n\n\
                    Context:\n{}",
                    context
                ).into(),
                name: None,
            };

            let user_message = ChatCompletionRequestUserMessage {
                content: query.into(),
                name: None,
            };

            let request = CreateChatCompletionRequest {
                model: self.model.clone(),
                messages: vec![
                    ChatCompletionRequestMessage::System(system_message),
                    ChatCompletionRequestMessage::User(user_message),
                ],
                stream: Some(true),
                max_tokens: Some(1000),
                temperature: Some(0.7),
                ..Default::default()
            };

            let mut stream = client.chat().create_stream(request).await?;

            while let Some(result) = stream.next().await {
                match result {
                    Ok(response) => {
                        for choice in response.choices {
                            if let Some(delta) = choice.delta.content {
                                self.send_response(&tx, RagResponse {
                                    message_type: "content".to_string(),
                                    content: Some(delta),
                                    citations: None,
                                }).await?;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error in streaming response: {}", e);
                        self.send_response(&tx, RagResponse {
                            message_type: "error".to_string(),
                            content: Some(format!("Error generating response: {}", e)),
                            citations: None,
                        }).await?;
                        break;
                    }
                }
            }

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

    // Factory function to create appropriate service
    pub fn create_rag_service(use_local: bool) -> Result<RagService, Box<dyn std::error::Error + Send + Sync>> {
        if use_local {
            match RagService::new_local() {
                Ok(service) => {
                    info!("Successfully initialized local RAG service");
                    Ok(service)
                }
                Err(e) => {
                    warn!("Failed to initialize local RAG service: {}, falling back to OpenAI", e);
                    Ok(RagService::new_openai())
                }
            }
        } else {
            Ok(RagService::new_openai())
        }
    }
}
