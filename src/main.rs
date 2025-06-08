use actix_web::{web, App, HttpServer, HttpResponse, error::ErrorInternalServerError};
use serde::{Deserialize, Serialize};
use log::{info, error};

mod llm;
mod tools;
mod handler;

use tools::WebSearchClient;
use handler::{QueryHandler, query_handler::ChatRequest};

#[derive(Deserialize)]
struct SearchRequest {
    query: String,
    count: Option<usize>,
}

async fn handle_chat(
    req: web::Json<ChatRequest>,
    handler: web::Data<QueryHandler>,
) -> Result<HttpResponse, actix_web::Error> {
    handler.handle_chat(req).await
}

async fn search(
    request: web::Json<SearchRequest>,
    web_search_client: web::Data<WebSearchClient>,
) -> Result<HttpResponse, actix_web::Error> {
    info!("Received search request with query: {}", request.query);
    
    let count = request.count.unwrap_or(5);
    let results = web_search_client
        .search(request.query.clone(), count)
        .await
        .map_err(|e| {
            error!("Web search error: {:?}", e);
            ErrorInternalServerError(e.to_string())
        })?;
    
    info!("Found {} search results", results.len());
    Ok(HttpResponse::Ok().json(results))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger with default (info) level
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    info!("Starting chat server...");
    
    // Create handlers
    let query_handler = web::Data::new(QueryHandler::new());
    let web_search_client = web::Data::new(WebSearchClient::new());
    
    info!("Server will be available at http://127.0.0.1:8080");
    
    HttpServer::new(move || {
        App::new()
            .app_data(query_handler.clone())
            .app_data(web_search_client.clone())
            .route("/chat", web::post().to(handle_chat))
            .route("/search", web::post().to(search))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
