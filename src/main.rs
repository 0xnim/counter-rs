use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use redis::{Client, Commands};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::env;

struct AppState {
    redis: Client,
    download_counts: Mutex<HashMap<String, i64>>,
}

#[get("/download/{token}")]
async fn download(web::Path(token): web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    // Increment the download count for the download token
    let mut download_counts = data.download_counts.lock().unwrap();
    let count = download_counts.entry(token.clone()).or_insert(0);
    *count += 1;

    // Return the download count as the response
    HttpResponse::Ok().body(count.to_string())
}

#[get("/read/{token}")]
async fn read(web::Path(token): web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    // Try to retrieve the download count from the cache
    let mut download_counts = data.download_counts.lock().unwrap();
    if let Some(download_count) = download_counts.get(&token) {
        return HttpResponse::Ok().body(download_count.to_string());
    }

    // If not found in the cache, retrieve the download count from Redis
    let mut conn = data.redis.get_connection().expect("Failed to get Redis connection");
    let download_count: i64 = conn.get(&token).unwrap_or(0);

    // Store the download count in the cache
    download_counts.insert(token.clone(), download_count);

    // Return the download count as the response
    HttpResponse::Ok().body(download_count.to_string())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Read the Redis URL from the environment variable
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL not set in .env file");

    // Create a Redis client
    let redis = Client::open(redis_url).expect("Failed to connect to Redis server");

    // Create the App state with the Redis client and the download counts HashMap
    let app_state = web::Data::new(AppState {
        redis,
        download_counts: Mutex::new(HashMap::new()),
    });

    // Start the Actix HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(download)
            .service(read)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
