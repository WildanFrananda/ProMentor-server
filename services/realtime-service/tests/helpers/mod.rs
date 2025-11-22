use actix_web::{test, web, App};
use realtime_service::api::ws_handler;
use realtime_service::services::session_manager::SessionManager;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;
use std::time::Duration;

pub mod mock_nats;

/// Test WebSocket client for integration testing
pub struct TestWsClient {
    pub url: String,
    pub session_id: Uuid,
}

impl TestWsClient {
    pub fn new(port: u16, session_id: Uuid) -> Self {
        Self {
            url: format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id),
            session_id,
        }
    }

    /// Connect to WebSocket endpoint
    pub async fn connect(&self) -> Result<
        (
            tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
            tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>,
        ),
        tokio_tungstenite::tungstenite::Error,
    > {
        connect_async(&self.url).await
    }

    /// Send a chat message
    pub async fn send_chat_message(
        ws: &mut tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        content: &str,
    ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        let msg = serde_json::json!({
            "msg_type": "text",
            "content": content
        });
        ws.send(Message::Text(msg.to_string())).await
    }

    /// Receive message with timeout
    pub async fn receive_with_timeout(
        ws: &mut tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        timeout: Duration,
    ) -> Result<Option<Message>, tokio::time::error::Elapsed> {
        tokio::time::timeout(timeout, ws.next()).await
    }
}

/// Create a test app with SessionManager
pub fn create_test_app() -> (
    actix_web::test::TestServer,
    web::Data<SessionManager>,
) {
    let manager = web::Data::new(SessionManager::new());
    let manager_clone = manager.clone();

    let server = test::start(move || {
        App::new()
            .app_data(manager_clone.clone())
            .route("/v1/ws/{session_id}", web::get().to(ws_handler::ws_route))
    });

    (server, manager)
}

/// Generate random session ID
pub fn random_session_id() -> Uuid {
    Uuid::new_v4()
}

/// Generate random connection ID
pub fn random_conn_id() -> usize {
    rand::random()
}

/// Assert message received within timeout
#[macro_export]
macro_rules! assert_receives_message {
    ($ws:expr, $timeout:expr) => {{
        let result = tokio::time::timeout($timeout, $ws.next())
            .await
            .expect("Should receive message within timeout");
        result.expect("Should have message")
    }};
}

/// Assert no message received within timeout
#[macro_export]
macro_rules! assert_no_message {
    ($ws:expr, $timeout:expr) => {{
        let result = tokio::time::timeout($timeout, $ws.next()).await;
        assert!(result.is_err(), "Should not receive any message");
    }};
}

/// Create malicious payload for security testing
pub fn create_xss_payload() -> String {
    r#"<script>alert('XSS')</script>"#.to_string()
}

pub fn create_sql_injection_payload() -> String {
    r#"'; DROP TABLE users; --"#.to_string()
}

pub fn create_large_payload(size_mb: usize) -> String {
    "A".repeat(size_mb * 1024 * 1024)
}

pub fn create_unicode_bomb() -> String {
    // Zalgo text that can cause rendering issues
    "H̴̡̢̨̧̛̛̖̠̗͓͇͙͓͚̺̦̜̮̝̱̱̱̠̠̙̺̩͉̺̜̩̰̺̜̤̠̙̫̰̭̮̗̰͇͕̪̪̪̦̼̼̺̺̙̙̺̪̺̺̙̠̺̼͖̦̞͓̟̩̟̫̞͖͚̺͖̼̦̫͔̟͙͕͕͕͙͙͖̩͙̩̩̞̩̺̺̠̺̦̪͖͙̠͙̪̪̩̪̩̼͚̼͚̺̺̪͙̪̪̠̪̪̠̦̦̫̫͕͕̩̩̺̪̠͓͇̫͓̺͔̩͚̺͙̺͓̪̫̫̫̫͓̦̪̺̼͚̙͔̫̫͖͙͙͙̺̫͙̫̫͙̺͙̫͙̠̪̪͖͙̪̪̪̠̺̠".to_string()
}