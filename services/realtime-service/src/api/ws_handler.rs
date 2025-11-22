use crate::{
    auth::jwt,
    events::nats_publisher::NatsPublisher,
    model::chat_message::{BroadcastMessage, ChatMessage, SenderInfo},
    services::session_manager::{Connection, SessionManager}
};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::StreamExt;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::interval;
use tokio::sync::mpsc;

#[derive(Deserialize)]
pub struct WsConnectQuery {
    token: String
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

pub async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    session_id: web::Path<uuid::Uuid>,
    query: web::Query<WsConnectQuery>,
    manager: web::Data<SessionManager>,
    publisher: web::Data<NatsPublisher>
) -> Result<HttpResponse, Error> {
    let claims = match jwt::validate_token(&query.token) {
        Ok(claims) => claims,
        Err(e) => {
            eprintln!("Token validation failed: {:?}", e);
            return Ok(HttpResponse::Unauthorized().body("Token invalid or expired"));
        }
    };

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;
    let session_id = session_id.into_inner();
    let conn_id = rand::random::<usize>();
    let (tx, mut rx) = mpsc::channel::<String>(16);

    manager.insert(session_id, conn_id, Connection { sender: tx, user_info: claims.clone() });
    
    actix_web::rt::spawn(async move {
        let mut interval = interval(HEARTBEAT_INTERVAL);

        loop {
            tokio::select! {
                Some(Ok(msg)) = msg_stream.next() => {
                    println!("📨 Received message from conn_id={}: {:?}", conn_id, msg);
                    
                    match msg {
                        Message::Text(text) => {
                            println!("📝 Text message received: {}", text);
                            
                            match serde_json::from_str::<ChatMessage>(&text.to_string()) {
                                Ok(chat_msg) => {
                                    println!("✅ Parsed ChatMessage: {:?}", chat_msg);
                                    if let Some(sender_info) = manager.get_user_info(session_id, conn_id) {
                                        publisher.publish_chat_message(session_id, &sender_info, &chat_msg.content).await;

                                        let broadcast_msg = BroadcastMessage {
                                            r#type: "chat_message".to_string(),
                                            sender: SenderInfo {
                                                id: sender_info.sub,
                                                name: sender_info.name,
                                            },
                                            content: chat_msg.content,
                                        };
                                        
                                        let broadcast_payload = serde_json::to_string(&broadcast_msg)
                                            .unwrap_or_else(|_| "{}".to_string());
                                        
                                        println!("📡 Broadcasting message: {}", broadcast_payload);
                                        manager.broadcast_message(session_id, &broadcast_payload, Some(conn_id));
                                    } else {
                                        eprintln!("❌ Could not find sender info for conn_id={}", conn_id);
                                    }
                                },
                                Err(e) => {
                                    eprintln!("❌ Failed to parse ChatMessage: {:?}", e);
                                    eprintln!("   Raw message was: {}", text);
                                }
                            }
                        },
                        Message::Ping(bytes) => { 
                            println!("🏓 Ping received from conn_id={}", conn_id);
                            if session.pong(&bytes).await.is_err() { 
                                eprintln!("❌ Failed to send pong to conn_id={}", conn_id);
                                break; 
                            } 
                        },
                        Message::Pong(_) => {},
                        Message::Close(reason) => {
                            println!("👋 Close message received from conn_id={}: {:?}", conn_id, reason);
                            break;
                        },
                        _ => {
                            println!("❓ Unknown message type from conn_id={}", conn_id);
                        }
                    }
                }

                Some(msg_to_send) = rx.recv() => {
                    println!("📤 Sending message to conn_id={}: {}", conn_id, msg_to_send);
                    if session.text(msg_to_send).await.is_err() {
                        eprintln!("❌ Failed to send message to conn_id={}", conn_id);
                        break;
                    } else {
                        println!("✅ Message sent successfully to conn_id={}", conn_id);
                    }
                }

                _ = interval.tick() => {
                    if session.ping(b"").await.is_err() {
                        eprintln!("❌ Failed to send heartbeat to conn_id={}", conn_id);
                        break;
                    }
                }
            }
        }

        manager.remove(session_id, conn_id);
        println!("🔌 Connection {} closed", conn_id);
    });

    return Ok(response);
}