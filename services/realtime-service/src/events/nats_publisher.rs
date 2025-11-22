use async_nats::{connect, Client, Error};
use serde::Serialize;
use serde_json::to_vec;
use std::env;
use uuid::Uuid;
use crate::auth::jwt::Claims;

#[derive(Serialize)]
struct ChatMessageReceivedEvent<'a> {
    event_type: &'static str,
    session_id: Uuid,
    user_id: Uuid,
    user_name: &'a str,
    content: &'a str
}

pub struct NatsPublisher {
    pub client: Client
}

impl NatsPublisher {
    pub async fn new() -> Result<Self, Error> {
        let nats_url = env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let client = connect(&nats_url).await?;
        println!("NATS publisher connected to realtime-service");
        Ok(NatsPublisher { client })
    }

    pub async fn publish_chat_message(&self, session_id: Uuid, sender_info: &Claims, content: &str) {
        let event = ChatMessageReceivedEvent {
            event_type: "chat.message.received",
            session_id,
            user_id: sender_info.sub,
            user_name: &sender_info.name,
            content
        };

        match to_vec(&event) {
            Ok(payload) => {
                let subject = format!("chat.message.received.{}", session_id);
                if let Err(e) = self.client.publish(subject, payload.into()).await {
                    eprintln!("Failed to publish chat message event: {}", e);
                } else {
                    println!("Published chat message event for session: {}", session_id);
                }
            }
            Err(e) => {
                eprintln!("Failed to serialize chat message event: {}", e);
            }
        }
    }
}