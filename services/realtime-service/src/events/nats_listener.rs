use actix_web::web;
use async_nats::Client;
use futures::StreamExt;
use serde::Deserialize;
use std::env;
use uuid::Uuid;

use crate::services::session_manager::SessionManager;

#[derive(Debug, Deserialize)]
struct EventPayload {
    event_type: String,
    session_id: Uuid
}

pub async fn run_nats_listener(manager: web::Data<SessionManager>) {
    let nats_url = env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());

    match async_nats::connect(&nats_url).await {
        Ok(client) => {
            println!("Connected to NATS in {}", nats_url);
            subscribe_to_subject(client, manager).await;
        }
        Err(e) => {
            println!("Failed to connect to NATS: {}", e);
        }
    }
}

async fn subscribe_to_subject(client: Client, manager: web::Data<SessionManager>) {
    let subjects = vec!["session.created", "session.joined"];

    for subject in subjects {
        match client.subscribe(subject.to_string()).await {
            Ok(mut sub) => {
                println!("Subscribed to subject: {}", subject);
                let manager_clone = manager.clone();

                tokio::spawn(async move {
                    while let Some(msg) = sub.next().await {
                        match serde_json::from_slice::<EventPayload>(&msg.payload) {
                            Ok(event) => {
                                println!(
                                    "Received event: {:?} for session: {}",
                                    event.event_type, event.session_id
                                );

                                let broadcast_msg = serde_json::json!({
                                    "type": event.event_type,
                                    "sessionId": event.session_id
                                }).to_string();

                                manager_clone.broadcast_message(event.session_id, &broadcast_msg, None);
                            }
                            Err(e) => {
                                println!("Failed to parse event payload: {}", e);
                            }
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("Failed to subscribe to subject: '{}': {}", subject, e);
            }
        }
    }
}
