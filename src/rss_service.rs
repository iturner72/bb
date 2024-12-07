#[cfg(feature = "ssr")]
pub mod server {
    use feed_rs::parser;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use std::error::Error;
    use std::convert::Infallible;
    use axum::response::sse::Event;
    use async_openai::{
        config::OpenAIConfig,
        types::{
            CreateChatCompletionRequest,
            ChatCompletionRequestSystemMessage,
            ChatCompletionRequestUserMessage,
            ResponseFormat,
        },
        Client,
    };

    use crate::server_fn::RssProgressUpdate;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct FeedResult {
        pub company: String,
        pub new_posts: i32,
        pub skipped_posts: i32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct FeedLink {
        company: String,
        link: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct PostInsights {
        summary: String,
        buzzwords: Vec<String>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct BlogPost {
        published_at: String,
        company: String,
        title: String,
        link: String,
        description: Option<String>,
        summary: Option<String>,
        full_text: Option<String>,
        buzzwords: Option<Vec<String>>,
    }

    pub async fn process_feeds_with_progress(
        progress_sender: tokio::sync::mpsc::Sender<Result<Event, Infallible>>
    ) -> Result<(), Box<dyn Error>> {
        let supabase = crate::supabase::get_client();
        let openai = Client::with_config(OpenAIConfig::default());
        
        let response = supabase
            .from("links")
            .select("company,link")
            .execute()
            .await?;
        
        let feed_links: Vec<FeedLink> = serde_json::from_str(&response.text().await?)?;
        
        for link_info in feed_links {
            let mut company_progress = RssProgressUpdate {
                company: link_info.company.clone(),
                status: "processing".to_string(),
                new_posts: 0,
                skipped_posts: 0,
                current_post: None,
            };
            
            // Send initial company status
            progress_sender.send(
                company_progress.clone().into_event()
            ).await?;
            
            // Process feed entries
            let response = reqwest::get(&link_info.link).await?;
            let content = response.bytes().await?;
            let feed = parser::parse(&content[..])?;

            for entry in feed.entries {
                let title = entry.title
                    .as_ref()
                    .map(|t| t.content.clone())
                    .unwrap_or_else(|| "Untitled".to_string());

                company_progress.current_post = Some(title.clone());
                progress_sender.send(company_progress.clone().into_event()).await?;
                
                let entry_link = entry.links
                    .first()
                    .map(|l| l.href.clone())
                    .unwrap_or_else(String::new);
                
                let existing = supabase
                    .from("poasts")
                    .select("*")
                    .eq("link", &entry_link)
                    .execute()
                    .await?;
                
                let existing_json: Value = serde_json::from_str(&existing.text().await?)?;
                if !existing_json.as_array().unwrap_or(&Vec::new()).is_empty() {
                    log::info!("Skipping existing post: {} from {}", title, link_info.company);
                    company_progress.skipped_posts += 1;
                    progress_sender.send(company_progress.clone().into_event()).await?;
                    continue;
                }

                // Process new post...
                log::info!("Processing new post: {} from {}", title, link_info.company);
                let full_text = scrape_page(&entry_link).await?;
                let insights = get_post_insights(&openai, &title, &full_text).await?;
                
                // Insert post...
                let post = BlogPost {
                    published_at: entry.published
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
                    company: link_info.company.clone(),
                    title: title.clone(),
                    link: entry_link,
                    description: entry.summary.map(|s| s.content),
                    summary: Some(insights.summary),
                    full_text: Some(full_text),
                    buzzwords: Some(insights.buzzwords),
                };

                let post_json = serde_json::to_string(&post)?;
                supabase
                    .from("poasts")
                    .insert(post_json)
                    .execute()
                    .await?;

                company_progress.new_posts += 1;
                progress_sender.send(company_progress.clone().into_event()).await?;
            }
            
            // Send final company status
            company_progress.status = "completed".to_string();
            company_progress.current_post = None;
            progress_sender.send(company_progress.into_event()).await?;
        }
        
        Ok(())
    }

    async fn scrape_page(url: &str) -> Result<String, Box<dyn Error>> {
        let response = reqwest::get(url).await?.text().await?;
        
        let document = scraper::Html::parse_document(&response);
        let body_selector = scraper::Selector::parse("body").unwrap();
        let text: String = document
            .select(&body_selector)
            .flat_map(|element| element.text())
            .collect();
        
        Ok(text)
    }

    async fn get_post_insights(
        client: &Client<OpenAIConfig>,
        title: &str,
        full_text: &str,
    ) -> Result<PostInsights, Box<dyn Error>> {
        let system_message = ChatCompletionRequestSystemMessage {
            content: "You are a helpful assistant.".into(),
            name: None,
        };

        let user_message = ChatCompletionRequestUserMessage {
            content: format!(
                "Create a one-line description for a technical blog post based on the title and full text I provide you. \
                Also, give me a list of the top 5 most important buzzwords from the same. \
                Respond only in JSON, using 'summary' and 'buzzwords' as the keys.\n\n\
                Title: '{}'\n\nFull Text: '{}'",
                title, full_text
            ).into(),
            name: None,
        };

        let request = CreateChatCompletionRequest {
            model: "gpt-3.5-turbo-0125".to_string(),
            messages: vec![
                system_message.into(),
                user_message.into(),
            ],
            response_format: Some(ResponseFormat::JsonObject),
            max_tokens: Some(200),
            ..Default::default()
        };

        let response = client.chat().create(request).await?;
        let content = response.choices[0].message.content.clone().unwrap_or_default();
        
        let insights: PostInsights = serde_json::from_str(&content)?;
        Ok(insights)
    }
}

#[cfg(not(feature = "ssr"))]
pub mod server {}
