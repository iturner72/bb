#[cfg(feature = "ssr")]
#[allow(clippy::module_inception)]
pub mod rag {
    use async_openai::{
        config::OpenAIConfig,
        Client,
    };
    use axum::response::sse::Event;
    use log::info;
    use serde::{Deserialize, Serialize};
    use std::convert::Infallible;
    use tokio::sync::mpsc;
    use anyhow::Result;
    use anyhow::anyhow;

    use crate::components::search::SearchType;

    pub use crate::rag_service::rag::rag::enhanced_rag::enhanced_rag::EnhancedRagService;

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

    pub struct RagService {
        _client: Client<OpenAIConfig>,
        _model: String,
    }

    impl Default for RagService {
        fn default() -> Self {
            Self::new()        
        }
    }

    impl RagService {
        pub fn new() -> Self {
            let _client = Client::new();
            Self {
                _client,
                _model: "gpt-3.5-turbo".to_string(),
            }
        }

        pub async fn process_query(
            &self,
            query: String,
            search_type: SearchType,
            tx: mpsc::Sender<Result<Event, Infallible>>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let enhanced_service = EnhancedRagService::new();
            enhanced_service.process_query(query, search_type, tx).await
        }
    }

    // Placeholder for future local LLM implementation
    pub struct LocalRagService {
        // TODO: Add local model fields when ready
        // model: Option<LocalLLMModel>,
    }

    impl LocalRagService {
        pub fn new() -> Result<Self> {
            // TODO: Initialize local model
            info!("Local RAG service not yet implemented - falling back to OpenAI");
            Err(anyhow!("Local RAG service not yet implemented"))
        }

        pub async fn process_query(
            &self,
            _query: String,
            _tx: mpsc::Sender<Result<Event, Infallible>>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            // TODO: Implement local RAG processing
            // This would follow similar pattern to RagService but use local model
            Err(Box::new(std::io::Error::other("Local RAG service not yet implemented")))
        }
    }

    mod enhanced_rag {
        #[cfg(feature = "ssr")]
        pub mod enhanced_rag {
            use async_openai::{
                config::OpenAIConfig,
                types::{
                    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
                    ChatCompletionRequestUserMessage, CreateChatCompletionRequest,
                    ChatCompletionTool, ChatCompletionToolType, FunctionObject,
                    ChatCompletionMessageToolCall, ChatCompletionToolChoiceOption,
                },
                Client,
            };
            use axum::response::sse::Event;
            use futures::StreamExt;
            use log::{error, info, debug};
            use serde::{Deserialize, Serialize};
            use serde_json::json;
            use std::convert::Infallible;
            use tokio::sync::mpsc;
            use anyhow::Result;
        
            use crate::components::poasts::{semantic_search, get_poasts, PostFilter, Poast};
            use crate::rag_service::rag::rag::RagResponse;
            use crate::components::search::SearchType;
            use crate::rag_service::rag::rag::Citation;
        
            #[derive(Debug, Serialize, Deserialize)]
            pub struct GetLatestPostsArgs {
                pub company: Option<String>,
                pub limit: Option<u32>,
            }
        
            #[derive(Debug, Serialize, Deserialize)]
            pub struct GetEarliestPostsArgs {
                pub company: Option<String>,
                pub limit: Option<u32>,
            }
        
            #[derive(Debug, Serialize, Deserialize)]
            pub struct SearchPostsByCompanyArgs {
                pub company: String,
                pub search_term: Option<String>,
                pub limit: Option<u32>,
            }
        
            #[derive(Debug, Serialize, Deserialize)]
            pub struct SemanticSearchArgs {
                pub query: String,
                pub search_type: Option<String>, // "openai" or "local"
                pub limit: Option<u32>,
            }
        
            pub struct EnhancedRagService {
                client: Client<OpenAIConfig>,
                model: String,
            }

            impl Default for EnhancedRagService {
                fn default() -> Self {
                    Self::new()        
                }
            }
        
            impl EnhancedRagService {
                pub fn new() -> Self {
                    let client = Client::new();
                    Self {
                        client,
                        model: "gpt-4o-mini".to_string(), // Use a model that supports function calling
                    }
                }
        
                pub async fn process_query(
                    &self,
                    query: String,
                    search_type: SearchType,
                    tx: mpsc::Sender<Result<Event, Infallible>>,
                ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    info!("Processing enhanced RAG query: {}", query);
        
                    // Step 1: Send initial status
                    self.send_status(&tx, "Analyzing your query...").await?;
                    debug!("analyzing query");
        
                    // Step 2: Define available functions
                    let tools = self.get_function_definitions();

                    debug!("available tools: {:?}", tools);
        
                    // Step 3: Let OpenAI decide which function(s) to call
                    let function_calls = self.determine_functions(&query, tools.clone()).await?;

                    debug!("function calls tools: {:?}", tools);
        
                    if function_calls.is_empty() {
                        // Fallback to semantic search if no functions are called
                        self.send_status(&tx, "Performing semantic search...").await?;
                        return self.fallback_semantic_search(query, search_type, tx).await;
                    }
        
                    // Step 4: Execute the function calls
                    let mut all_posts = Vec::new();
                    let mut function_results = Vec::new();
        
                    for tool_call in function_calls {
                        self.send_status(&tx, &format!("Executing: {}", tool_call.function.name)).await?;
                        
                        let (posts, result_description) = self.execute_function_call(&tool_call, search_type).await?;
                        all_posts.extend(posts);
                        function_results.push(result_description);
                    }
        
                    // Step 5: Remove duplicates and limit results
                    all_posts.sort_by(|a, b| b.published_at.cmp(&a.published_at));
                    all_posts.dedup_by(|a, b| a.id == b.id);
                    all_posts.truncate(20); // Limit to 20 total posts
        
                    if all_posts.is_empty() {
                        self.send_error(&tx, "No posts found matching your criteria.").await?;
                        return Ok(());
                    }
        
                    // Step 6: Send citations
                    let citations = self.create_citations(&all_posts);
                    self.send_citations(&tx, citations).await?;
        
                    // Step 7: Generate contextual response
                    self.send_status(&tx, "Generating response...").await?;
                    let context = self.create_context(&all_posts, &function_results);
                    self.generate_streaming_response(query, context, tx).await?;
        
                    Ok(())
                }
        
                fn get_function_definitions(&self) -> Vec<ChatCompletionTool> {
                    vec![
                        ChatCompletionTool {
                            r#type: ChatCompletionToolType::Function,
                            function: FunctionObject {
                                name: "get_latest_posts".to_string(),
                                description: Some("Get the most recent blog posts, optionally filtered by company. Use this when users ask for 'latest', 'newest', 'recent' posts.".to_string()),
                                parameters: Some(json!({
                                    "type": "object",
                                    "properties": {
                                        "company": {
                                            "type": "string",
                                            "description": "Filter posts by this company name (optional)"
                                        },
                                        "limit": {
                                            "type": "integer",
                                            "description": "Number of posts to return (default: 10, max: 20)",
                                            "minimum": 1,
                                            "maximum": 20
                                        }
                                    }
                                })),
                                strict: None,
                            },
                        },
                        ChatCompletionTool {
                            r#type: ChatCompletionToolType::Function,
                            function: FunctionObject {
                                name: "get_earliest_posts".to_string(),
                                description: Some("Get the oldest blog posts, optionally filtered by company. Use this when users ask for 'earliest', 'oldest', 'first' posts.".to_string()),
                                parameters: Some(json!({
                                    "type": "object",
                                    "properties": {
                                        "company": {
                                            "type": "string",
                                            "description": "Filter posts by this company name (optional)"
                                        },
                                        "limit": {
                                            "type": "integer",
                                            "description": "Number of posts to return (default: 10, max: 20)",
                                            "minimum": 1,
                                            "maximum": 20
                                        }
                                    }
                                })),
                                strict: None,
                            },
                        },
                        ChatCompletionTool {
                            r#type: ChatCompletionToolType::Function,
                            function: FunctionObject {
                                name: "search_posts_by_company".to_string(),
                                description: Some("Search for posts from a specific company, optionally with a search term or phrase. Use this when users specify a particular company.".to_string()),
                                parameters: Some(json!({
                                    "type": "object",
                                    "properties": {
                                        "company": {
                                            "type": "string",
                                            "description": "The company name to filter by"
                                        },
                                        "search_term": {
                                            "type": "string",
                                            "description": "Optional search term or phrase to filter posts within the company"
                                        },
                                        "limit": {
                                            "type": "integer",
                                            "description": "Number of posts to return (default: 10, max: 20)",
                                            "minimum": 1,
                                            "maximum": 20
                                        }
                                    },
                                    "required": ["company"]
                                })),
                                strict: None,
                            },
                        },
                        ChatCompletionTool {
                            r#type: ChatCompletionToolType::Function,
                            function: FunctionObject {
                                name: "semantic_search_posts".to_string(),
                                description: Some("Perform semantic search across all posts to find content similar to the query. Use this for conceptual or thematic searches.".to_string()),
                                parameters: Some(json!({
                                    "type": "object",
                                    "properties": {
                                        "query": {
                                            "type": "string",
                                            "description": "The search query for semantic matching"
                                        },
                                        "search_type": {
                                            "type": "string",
                                            "enum": ["openai", "local"],
                                            "description": "Type of semantic search to use"
                                        },
                                        "limit": {
                                            "type": "integer",
                                            "description": "Number of posts to return (default: 10, max: 20)",
                                            "minimum": 1,
                                            "maximum": 20
                                        }
                                    },
                                    "required": ["query"]
                                })),
                                strict: None,
                            },
                        },
                    ]
                }
        
                async fn determine_functions(
                    &self,
                    query: &str,
                    tools: Vec<ChatCompletionTool>,
                ) -> Result<Vec<ChatCompletionMessageToolCall>, Box<dyn std::error::Error + Send + Sync>> {
                    let system_message = ChatCompletionRequestSystemMessage {
                        content: "You are a helpful assistant that helps users blog posts. \
                        Analyze the user's query and determine which function(s) to call to best answer their question. \
                        You can call multiple functions if needed (e.g., to compare latest posts from different companies). \
                        Always try to call at least one function to retrieve posts.".into(),
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
                        tools: Some(tools),
                        tool_choice: Some(ChatCompletionToolChoiceOption::Auto),
                        temperature: Some(0.1),
                        ..Default::default()
                    };
        
                    let response = self.client.chat().create(request).await?;
        
                    Ok(response.choices
                        .into_iter()
                        .next()
                        .and_then(|choice| choice.message.tool_calls)
                        .unwrap_or_default())
                }
        
                async fn execute_function_call(
                    &self,
                    tool_call: &ChatCompletionMessageToolCall,
                    _search_type: SearchType,
                ) -> Result<(Vec<Poast>, String), Box<dyn std::error::Error + Send + Sync>> {
                    debug!("Executing function: {} with args: {}", tool_call.function.name, tool_call.function.arguments);
        
                    match tool_call.function.name.as_str() {
                        "get_latest_posts" => {
                            let args: GetLatestPostsArgs = serde_json::from_str(&tool_call.function.arguments)?;
                            let posts = self.get_latest_posts(args.company.clone(), args.limit).await?;
                            let description = match args.company {
                                Some(company) => format!("Found {} latest posts from {}", posts.len(), company),
                                None => format!("Found {} latest posts from all companies", posts.len()),
                            };
                            Ok((posts, description))
                        }
                        "get_earliest_posts" => {
                            let args: GetEarliestPostsArgs = serde_json::from_str(&tool_call.function.arguments)?;
                            let posts = self.get_earliest_posts(args.company.clone(), args.limit).await?;
                            let description = match args.company {
                                Some(company) => format!("Found {} earliest posts from {}", posts.len(), company),
                                None => format!("Found {} earliest posts from all companies", posts.len()),
                            };
                            Ok((posts, description))
                        }
                        "search_posts_by_company" => {
                            let args: SearchPostsByCompanyArgs = serde_json::from_str(&tool_call.function.arguments)?;
                            let posts = self.search_posts_by_company(&args.company, args.search_term.as_deref(), args.limit).await?;
                            let description = match args.search_term {
                                Some(term) => format!("Found {} posts from {} matching '{}'", posts.len(), args.company, term),
                                None => format!("Found {} posts from {}", posts.len(), args.company),
                            };
                            Ok((posts, description))
                        }
                        "semantic_search_posts" => {
                            let args: SemanticSearchArgs = serde_json::from_str(&tool_call.function.arguments)?;
                            let search_type = match args.search_type.as_deref().unwrap_or("openai") {
                                "local" => SearchType::LocalSemantic,
                                _ => SearchType::OpenAISemantic,
                            };
                            let mut posts = semantic_search(args.query.clone(), search_type).await
                                .map_err(|e| format!("Semantic search failed: {}", e))?;
                            if let Some(limit) = args.limit {
                                posts.truncate(limit as usize);
                            }
                            let description = format!("Found {} posts semantically similar to '{}'", posts.len(), args.query);
                            Ok((posts, description))
                        }
                        _ => Err(format!("Unknown function: {}", tool_call.function.name).into()),
                    }
                }
        
                async fn get_latest_posts(
                    &self,
                    company: Option<String>,
                    limit: Option<u32>,
                ) -> Result<Vec<Poast>, Box<dyn std::error::Error + Send + Sync>> {
                    let filter = PostFilter {
                        search_term: None,
                        company,
                    };
                    
                    let mut posts = get_poasts(Some(filter)).await
                        .map_err(|e| format!("Failed to get latest posts: {}", e))?;
                    
                    // Sort by published_at descending (newest first)
                    posts.sort_by(|a, b| b.published_at.cmp(&a.published_at));
                    
                    if let Some(limit) = limit {
                        posts.truncate(limit as usize);
                    } else {
                        posts.truncate(10);
                    }
                    
                    Ok(posts)
                }
        
                async fn get_earliest_posts(
                    &self,
                    company: Option<String>,
                    limit: Option<u32>,
                ) -> Result<Vec<Poast>, Box<dyn std::error::Error + Send + Sync>> {
                    let filter = PostFilter {
                        search_term: None,
                        company,
                    };
                    
                    let mut posts = get_poasts(Some(filter)).await
                        .map_err(|e| format!("Failed to get earliest posts: {}", e))?;
                    
                    // Sort by published_at ascending (oldest first)
                    posts.sort_by(|a, b| a.published_at.cmp(&b.published_at));
                    
                    if let Some(limit) = limit {
                        posts.truncate(limit as usize);
                    } else {
                        posts.truncate(10);
                    }
                    
                    Ok(posts)
                }
        
                async fn search_posts_by_company(
                    &self,
                    company: &str,
                    search_term: Option<&str>,
                    limit: Option<u32>,
                ) -> Result<Vec<Poast>, Box<dyn std::error::Error + Send + Sync>> {
                    let filter = PostFilter {
                        search_term: search_term.map(|s| s.to_string()),
                        company: Some(company.to_string()),
                    };
                    
                    let mut posts = get_poasts(Some(filter)).await
                        .map_err(|e| format!("Failed to search posts by company: {}", e))?;
                    
                    // Sort by published_at descending (newest first)
                    posts.sort_by(|a, b| b.published_at.cmp(&a.published_at));
                    
                    if let Some(limit) = limit {
                        posts.truncate(limit as usize);
                    } else {
                        posts.truncate(10);
                    }
                    
                    Ok(posts)
                }
        
                fn create_citations(&self, posts: &[Poast]) -> Vec<Citation> {
                    posts
                        .iter()
                        .enumerate()
                        .map(|(i, post)| Citation {
                            title: post.title.clone(),
                            company: post.company.clone(),
                            link: post.link.clone(),
                            published_at: post.published_at.clone(),
                            relevance_score: 1.0 - (i as f32 * 0.05), // Slight decrease in relevance
                        })
                        .collect()
                }
        
                fn create_context(&self, posts: &[Poast], function_results: &[String]) -> String {
                    let mut context = String::new();
                    
                    context.push_str("Function calls executed:\n");
                    for result in function_results {
                        context.push_str(&format!("- {}\n", result));
                    }
                    context.push('\n');
        
                    context.push_str("Retrieved blog posts:\n\n");
        
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
        
                async fn fallback_semantic_search(
                    &self,
                    query: String,
                    search_type: SearchType,
                    tx: mpsc::Sender<Result<Event, Infallible>>,
                ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let posts = semantic_search(query.clone(), search_type).await
                        .map_err(|e| format!("Semantic search failed: {}", e))?;
                    
                    if posts.is_empty() {
                        self.send_error(&tx, "No relevant posts found for your query.").await?;
                        return Ok(());
                    }
        
                    let citations = self.create_citations(&posts);
                    self.send_citations(&tx, citations).await?;
        
                    let context = self.create_context(&posts, &[]);
                    self.generate_streaming_response(query, context, tx).await?;
        
                    Ok(())
                }
        
                // Helper methods for sending different types of responses
                async fn send_status(
                    &self,
                    tx: &mpsc::Sender<Result<Event, Infallible>>,
                    status: &str,
                ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let response = RagResponse {
                        message_type: "status".to_string(),
                        content: Some(status.to_string()),
                        citations: None,
                    };
                    self.send_response(tx, response).await
                }
        
                async fn send_citations(
                    &self,
                    tx: &mpsc::Sender<Result<Event, Infallible>>,
                    citations: Vec<Citation>,
                ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let response = RagResponse {
                        message_type: "citations".to_string(),
                        content: None,
                        citations: Some(citations),
                    };
                    self.send_response(tx, response).await
                }
        
                async fn send_error(
                    &self,
                    tx: &mpsc::Sender<Result<Event, Infallible>>,
                    error: &str,
                ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let response = RagResponse {
                        message_type: "error".to_string(),
                        content: Some(error.to_string()),
                        citations: None,
                    };
                    self.send_response(tx, response).await
                }
        
                async fn send_response(
                    &self,
                    tx: &mpsc::Sender<Result<Event, Infallible>>,
                    response: RagResponse,
                ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let json = serde_json::to_string(&response)?;
                    tx.send(Ok(Event::default().data(json))).await
                        .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;
                    Ok(())
                }
        
                async fn generate_streaming_response(
                    &self,
                    query: String,
                    context: String,
                    tx: mpsc::Sender<Result<Event, Infallible>>,
                ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let system_message = ChatCompletionRequestSystemMessage {
                        content: format!(
                            "You are a helpful assistant that answers questions about blog posts. \
                            Use the provided context to answer the user's question. The context includes information about which \
                            functions were called and what posts were retrieved. \
                            
                            Always reference specific posts when relevant by mentioning the company name and post title. \
                            Be concise but informative. Format your response in markdown.\n\n\
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
                        max_completion_tokens: Some(1000),
                        temperature: Some(0.7),
                        ..Default::default()
                    };
        
                    let mut stream = self.client.chat().create_stream(request).await?;
        
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(response) => {
                                for choice in response.choices {
                                    if let Some(delta) = choice.delta.content {
                                        let response = RagResponse {
                                            message_type: "content".to_string(),
                                            content: Some(delta),
                                            citations: None,
                                        };
                                        self.send_response(&tx, response).await?;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error in streaming response: {}", e);
                                self.send_error(&tx, &format!("Error generating response: {}", e)).await?;
                                break;
                            }
                        }
                    }
        
                    // Send completion signal
                    let response = RagResponse {
                        message_type: "done".to_string(),
                        content: None,
                        citations: None,
                    };
                    self.send_response(&tx, response).await?;
        
                    Ok(())
                }
            }
        }
    }
}
