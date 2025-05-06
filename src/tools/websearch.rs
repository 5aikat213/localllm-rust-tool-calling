use log::{info, error};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use scraper::{Html, Selector};

#[derive(Debug, Clone, Copy)]
pub enum SearchEngine {
    DuckDuckGo,
}

impl Default for SearchEngine {
    fn default() -> Self {
        SearchEngine::DuckDuckGo
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub content: String,
    pub url: String,
}

#[derive(Error, Debug)]
pub enum WebSearchError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Failed to parse URL: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Search error: {0}")]
    SearchError(String),
}

pub struct WebSearchClient {
    client: reqwest::Client,
    engine: SearchEngine,
}

impl WebSearchClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
                .build()
                .unwrap(),
            engine: SearchEngine::default(),
        }
    }

    #[allow(dead_code)]
    pub fn with_engine(engine: SearchEngine) -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
                .build()
                .unwrap(),
            engine,
        }
    }

    pub async fn search(&self, query: String, count: usize) -> Result<Vec<SearchResult>, WebSearchError> {
        match self.engine {
            SearchEngine::DuckDuckGo => self.search_duckduckgo(&query, count).await,
        }
    }

    async fn search_duckduckgo(&self, query: &str, count: usize) -> Result<Vec<SearchResult>, WebSearchError> {
        info!("Performing DuckDuckGo search for query: {}", query);
        
        let search_url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let response = self.client
            .get(&search_url)
            .send()
            .await?
            .text()
            .await?;

        let document = Html::parse_document(&response);
        
        // DuckDuckGo search result selectors
        let result_selector = Selector::parse(".result").unwrap();
        let title_selector = Selector::parse(".result__title a").unwrap();
        let snippet_selector = Selector::parse(".result__snippet").unwrap();

        let mut results = Vec::new();
        
        for result in document.select(&result_selector).take(count) {
            if let (Some(title_elem), Some(snippet_elem)) = (
                result.select(&title_selector).next(),
                result.select(&snippet_selector).next()
            ) {
                let title = title_elem.text().collect::<String>();
                let content = snippet_elem.text().collect::<String>();
                let url = title_elem.value().attr("href").unwrap_or("").to_string();

                // Only add results with valid URLs
                if !url.is_empty() {
                    results.push(SearchResult {
                        title: title.trim().to_string(),
                        content: content.trim().to_string(),
                        url,
                    });
                }
            }
        }

        info!("Found {} DuckDuckGo search results", results.len());
        Ok(results)
    }

    #[allow(dead_code)]
    pub async fn fetch_page_content(&self, url: &str) -> Result<String, WebSearchError> {
        
        let response = self.client
            .get(url)
            .send()
            .await?
            .text()
            .await?;

        let document = scraper::Html::parse_document(&response);
        let selector = scraper::Selector::parse("p, h1, h2, h3, h4, h5, h6, article, section").unwrap();
        
        let content: String = document
            .select(&selector)
            .map(|element| element.text().collect::<String>())
            .collect::<Vec<String>>()
            .join("\n\n");

        Ok(content.trim().to_string())
    }
}
