mod api;
mod services;
mod model;
mod auth;
mod events {
    pub mod nats_listener;
    pub mod nats_publisher;
}
mod middleware;

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use async_nats::connect;
use services::session_manager::SessionManager;
use std::env;
use std::io::Result;

use crate::middleware::metrics::{metrics_handler, prometheus_middleware, register_metrics};

#[get("/health")]
async fn health_check() -> impl Responder {
    return HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "realtime-service"
    }));
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    register_metrics();

    let session_manager = web::Data::new(SessionManager::new());
    let nats_publisher = match events::nats_publisher::NatsPublisher::new().await {
        Ok(publisher) => web::Data::new(publisher),
        Err(e) => {
            eprintln!("Failed to create NATS publisher: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create NATS publisher: {}", e),
            ));
        }
    };

    tokio::spawn(events::nats_listener::run_nats_listener(session_manager.clone()));

    let port_str = env::var("APP_PORT").unwrap_or_else(|_| "8080".to_string());
    let port = port_str.parse::<u16>().unwrap();

    println!("Listening realtime-service on port {}", port);

    return HttpServer::new(move || {
        App::new()
            .wrap_fn(prometheus_middleware)
            .app_data(session_manager.clone())
            .app_data(nats_publisher.clone())
            .service(health_check)
            .route("/v1/ws/{session_id}", web::get().to(api::ws_handler::ws_route))
            .route("/metrics", web::get().to(metrics_handler()))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await;
}