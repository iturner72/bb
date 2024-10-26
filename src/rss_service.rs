#[cfg(feature = "ssr")]
pub mod server {
    use chrono::DateTime;
    use feed_rs::parser;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use std::error::Error;
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

    pub async fn process_feeds() -> Result<Vec<FeedResult>, Box<dyn Error>> {
        let supabase = crate::supabase::get_client();
        let openai = Client::with_config(OpenAIConfig::default());
        let mut results = Vec::new();
        
        let response = supabase
            .from("links")
            .select("company,link")
            .execute()
            .await?;
        
        let feed_links: Vec<FeedLink> = serde_json::from_str(&response.text().await?)?;

        for link_info in feed_links {
            let mut result = FeedResult {
                company: link_info.company.clone(),
                new_posts: 0,
                skipped_posts: 0,
            };
            
            match process_single_feed(supabase, &openai, &link_info, &mut result).await {
                Ok(_) => {
                    log::info!(
                        "Processed feed for {}: {} new posts, {} skipped",
                        result.company,
                        result.new_posts,
                        result.skipped_posts
                    );
                    results.push(result);
                },
                Err(e) => log::error!("Error processing feed {}: {}", link_info.company, e),
            }
        }
        
        Ok(results)
    }

    async fn process_single_feed(
        supabase: &postgrest::Postgrest,
        openai: &Client<OpenAIConfig>,
        link_info: &FeedLink,
        result: &mut FeedResult,
    ) -> Result<(), Box<dyn Error>> {
        let response = reqwest::get(&link_info.link).await?;
        let content = response.bytes().await?;
        let feed = parser::parse(&content[..])?;

        for entry in feed.entries {
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
                result.skipped_posts += 1;
                continue;
            }

            let title = entry.title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_else(|| "Untitled".to_string());

            let full_text = scrape_page(&entry_link).await?;
            let insights = get_post_insights(openai, &title, &full_text).await?;
            
            let post = BlogPost {
                published_at: entry.published
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
                company: link_info.company.clone(),
                title: entry.title.map(|t| t.content).unwrap_or_default(),
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

            result.new_posts += 1;
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
            max_tokens: Some(100),
            ..Default::default()
        };

        let response = client.chat().create(request).await?;
        let content = response.choices[0].message.content.clone().unwrap_or_default();
        
        let insights: PostInsights = serde_json::from_str(&content)?;
        Ok(insights)
    }

    fn extract_post_data(entry: &feed_rs::model::Entry, company: &str) -> Result<BlogPost, Box<dyn Error>> {
        let published = entry.published.unwrap_or(entry.updated.unwrap_or_default());
        let published_at = DateTime::from_timestamp(published.timestamp(), 0)
            .unwrap_or_default()
            .format("%Y-%m-%d")
            .to_string();

        Ok(BlogPost {
            published_at,
            company: company.to_string(),
            title: entry.title.as_ref().map(|t| t.content.clone()).unwrap_or_default(),
            link: entry.links.first().map(|l| l.href.clone()).unwrap_or_default(),
            description: entry.summary.as_ref().map(|s| s.content.clone()),
            summary: None,
            full_text: None,
            buzzwords: None,
        })
    }
}

#[cfg(not(feature = "ssr"))]
pub mod server {}
