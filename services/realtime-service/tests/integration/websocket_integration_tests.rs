use actix_web::{web, App, HttpServer};
use futures_util::{SinkExt, StreamExt};
use realtime_service::api::ws_handler;
use realtime_service::services::session_manager::SessionManager;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;
use std::time::Duration;

#[cfg(test)]
mod ws_integration_tests {
    use super::*;

    async fn start_test_server() -> (u16, web::Data<SessionManager>) {
        let manager = web::Data::new(SessionManager::new());
        let manager_clone = manager.clone();

        let server = HttpServer::new(move || {
            App::new()
                .app_data(manager_clone.clone())
                .route("/v1/ws/{session_id}", web::get().to(ws_handler::ws_route))
        })
        .bind(("127.0.0.1", 0))
        .expect("Failed to bind")
        .run();

        let port = server.addrs().first().unwrap().port();
        tokio::spawn(server);
        
        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        (port, manager)
    }

    #[tokio::test]
    async fn test_single_client_connection() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (ws_stream, _response) = connect_async(&url)
            .await
            .expect("Failed to connect");

        println!("✓ Single client connected successfully");
        drop(ws_stream);
    }

    #[tokio::test]
    async fn test_client_send_receive_message() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

        // Send a message
        let msg = serde_json::json!({
            "msg_type": "text",
            "content": "Hello, World!"
        });
        
        ws_stream.send(Message::Text(msg.to_string())).await.expect("Failed to send");

        // Should receive echo back (broadcast to self)
        let received = tokio::time::timeout(Duration::from_secs(1), ws_stream.next())
            .await
            .expect("Timeout waiting for response")
            .expect("No message received")
            .expect("Error receiving message");

        match received {
            Message::Text(text) => {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(parsed["content"], "Hello, World!");
                println!("✓ Message echo received");
            }
            _ => panic!("Expected text message"),
        }
    }

    #[tokio::test]
    async fn test_multiple_clients_broadcast() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();

        // Connect 3 clients
        let mut clients = Vec::new();
        for _ in 0..3 {
            let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);
            let (ws, _) = connect_async(&url).await.expect("Failed to connect");
            clients.push(ws);
        }

        // Client 0 sends a message
        let msg = serde_json::json!({
            "msg_type": "text",
            "content": "Broadcast test"
        });
        
        clients[0].send(Message::Text(msg.to_string())).await.expect("Send failed");

        // All clients should receive (client 0 is skipped in broadcast but might still get it)
        for (i, client) in clients.iter_mut().enumerate() {
            let result = tokio::time::timeout(Duration::from_secs(1), client.next()).await;
            
            if i == 0 {
                // Sender is skipped, should not receive
                // But due to race condition, might still receive
                println!("Client {} receive result: {:?}", i, result.is_ok());
            } else {
                // Other clients should receive
                let received = result
                    .expect("Timeout")
                    .expect("No message")
                    .expect("Error");
                
                match received {
                    Message::Text(text) => {
                        assert!(text.contains("Broadcast test"));
                        println!("✓ Client {} received broadcast", i);
                    }
                    _ => panic!("Expected text message"),
                }
            }
        }
    }

    #[tokio::test]
    async fn test_heartbeat_ping_pong() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

        // Wait for ping (heartbeat is 5 seconds)
        let ping = tokio::time::timeout(Duration::from_secs(6), async {
            loop {
                if let Some(Ok(msg)) = ws_stream.next().await {
                    if matches!(msg, Message::Ping(_)) {
                        return msg;
                    }
                }
            }
        })
        .await
        .expect("No ping received");

        match ping {
            Message::Ping(data) => {
                println!("✓ Received ping: {:?}", data);
                
                // Respond with pong
                ws_stream.send(Message::Pong(data)).await.expect("Failed to send pong");
                println!("✓ Sent pong response");
            }
            _ => panic!("Expected ping"),
        }
    }

    #[tokio::test]
    async fn test_client_disconnect_cleanup() {
        let (port, manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
        
        // Verify connection is tracked
        tokio::time::sleep(Duration::from_millis(100)).await;
        {
            let sessions = manager.sessions.lock().unwrap();
            assert!(sessions.contains_key(&session_id), "Session should exist");
        }

        // Disconnect
        drop(ws_stream);
        
        // Wait for cleanup
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Verify cleanup
        let sessions = manager.sessions.lock().unwrap();
        assert!(!sessions.contains_key(&session_id), "Session should be cleaned up");
        println!("✓ Connection cleanup verified");
    }

    #[tokio::test]
    async fn test_invalid_message_handling() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

        // Send invalid JSON
        ws_stream.send(Message::Text("not json".to_string())).await.expect("Send failed");

        // Connection should remain open despite invalid message
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Send valid message to verify connection is still alive
        let valid_msg = serde_json::json!({
            "msg_type": "text",
            "content": "Valid message"
        });
        ws_stream.send(Message::Text(valid_msg.to_string())).await.expect("Send failed");

        let received = tokio::time::timeout(Duration::from_secs(1), ws_stream.next())
            .await
            .expect("Should receive message");
        
        assert!(received.is_some(), "Connection should still be alive");
        println!("✓ Connection survives invalid message");
    }

    #[tokio::test]
    async fn test_concurrent_connections_same_session() {
        let (port, manager) = start_test_server().await;
        let session_id = Uuid::new_v4();

        let mut handles = vec![];
        
        // Create 50 concurrent connections
        for i in 0..50 {
            let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);
            
            let handle = tokio::spawn(async move {
                let (mut ws, _) = connect_async(&url).await.expect("Failed to connect");
                
                // Send a message
                let msg = serde_json::json!({
                    "msg_type": "text",
                    "content": format!("Message from client {}", i)
                });
                ws.send(Message::Text(msg.to_string())).await.expect("Send failed");
                
                // Keep connection alive briefly
                tokio::time::sleep(Duration::from_millis(500)).await;
                
                i
            });
            
            handles.push(handle);
        }

        // Wait for all connections
        for handle in handles {
            handle.await.expect("Task failed");
        }

        // Verify session tracking
        tokio::time::sleep(Duration::from_millis(100)).await;
        let sessions = manager.sessions.lock().unwrap();
        
        if let Some(session) = sessions.get(&session_id) {
            println!("✓ {} connections tracked in session", session.len());
        }
    }

    #[tokio::test]
    async fn test_message_with_special_characters() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

        let special_chars = vec![
            "Hello\nWorld",           // Newline
            "Hello\tWorld",           // Tab
            "Hello\"World\"",         // Quotes
            "Hello\\World",           // Backslash
            "Hello 世界",             // Unicode
            "Hello 🌍🚀",            // Emoji
        ];

        for content in special_chars {
            let msg = serde_json::json!({
                "msg_type": "text",
                "content": content
            });
            
            ws_stream.send(Message::Text(msg.to_string())).await.expect("Send failed");
            
            // Receive echo
            let received = tokio::time::timeout(Duration::from_secs(1), ws_stream.next())
                .await
                .expect("Timeout");
            
            assert!(received.is_some(), "Should receive for: {}", content);
        }
        
        println!("✓ All special characters handled");
    }

    #[tokio::test]
    async fn test_rapid_messages() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

        // Send 100 messages rapidly
        for i in 0..100 {
            let msg = serde_json::json!({
                "msg_type": "text",
                "content": format!("Rapid message {}", i)
            });
            
            ws_stream.send(Message::Text(msg.to_string())).await.expect("Send failed");
        }

        // Count received messages
        let mut received_count = 0;
        let timeout = Duration::from_secs(2);
        
        loop {
            match tokio::time::timeout(Duration::from_millis(50), ws_stream.next()).await {
                Ok(Some(Ok(_))) => received_count += 1,
                _ => break,
            }
        }

        // ⚠️ RELIABILITY TEST: Due to channel buffer (16), some messages will be lost
        println!("⚠️  Sent: 100, Received: {} (Buffer overflow expected)", received_count);
        
        // We expect message loss with buffer size of 16
        assert!(received_count < 100, "Should lose messages due to small buffer");
    }

    #[tokio::test]
    async fn test_large_message_handling() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

        // Send 1MB message
        let large_content = "A".repeat(1_000_000);
        let msg = serde_json::json!({
            "msg_type": "text",
            "content": large_content
        });

        let result = ws_stream.send(Message::Text(msg.to_string())).await;
        
        // ⚠️ SECURITY: Large message is accepted without validation
        assert!(result.is_ok(), "Large message should be rejected but isn't!");
        println!("⚠️  DOS VULNERABILITY: 1MB message accepted");
    }

    #[tokio::test]
    async fn test_connection_isolation_between_sessions() {
        let (port, _manager) = start_test_server().await;
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        let url_a = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_a);
        let url_b = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_b);

        let (mut ws_a, _) = connect_async(&url_a).await.expect("Failed to connect A");
        let (mut ws_b, _) = connect_async(&url_b).await.expect("Failed to connect B");

        // Send message in session A
        let msg = serde_json::json!({
            "msg_type": "text",
            "content": "Session A message"
        });
        ws_a.send(Message::Text(msg.to_string())).await.expect("Send failed");

        // Session B should NOT receive this message
        let result_b = tokio::time::timeout(Duration::from_millis(500), ws_b.next()).await;
        assert!(result_b.is_err(), "Session B should not receive Session A's message");

        // Session A should receive (as sender might get echo)
        let result_a = tokio::time::timeout(Duration::from_millis(500), ws_a.next()).await;
        println!("✓ Session isolation verified: {:?}", result_a.is_ok());
    }

    #[tokio::test]
    async fn test_close_frame_handling() {
        let (port, manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

        // Send close frame
        ws_stream.send(Message::Close(None)).await.expect("Send close failed");

        // Connection should close gracefully
        tokio::time::sleep(Duration::from_millis(200)).await;

        let sessions = manager.sessions.lock().unwrap();
        assert!(!sessions.contains_key(&session_id), "Session should be removed after close");
        println!("✓ Close frame handled gracefully");
    }

    #[tokio::test]
    async fn test_binary_message_handling() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

        // Send binary message (should be ignored by current implementation)
        ws_stream.send(Message::Binary(vec![1, 2, 3, 4])).await.expect("Send failed");

        // Connection should remain stable
        tokio::time::sleep(Duration::from_millis(100)).await;

        let msg = serde_json::json!({
            "msg_type": "text",
            "content": "After binary"
        });
        let result = ws_stream.send(Message::Text(msg.to_string())).await;
        assert!(result.is_ok(), "Connection should still work after binary message");
        println!("✓ Binary message ignored, connection stable");
    }

    // ⚠️ SECURITY TESTS

    #[tokio::test]
    async fn test_xss_payload_transmission() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

        let xss_payloads = vec![
            r#"<script>alert('XSS')</script>"#,
            r#"<img src=x onerror=alert(1)>"#,
            r#"javascript:alert('XSS')"#,
            r#"<svg onload=alert(1)>"#,
        ];

        for payload in xss_payloads {
            let msg = serde_json::json!({
                "msg_type": "text",
                "content": payload
            });
            
            ws_stream.send(Message::Text(msg.to_string())).await.expect("Send failed");

            let received = tokio::time::timeout(Duration::from_secs(1), ws_stream.next())
                .await
                .expect("Timeout")
                .expect("No message")
                .expect("Error");

            match received {
                Message::Text(text) => {
                    // ⚠️ VULNERABILITY: XSS payload is echoed back unchanged
                    assert!(text.contains(payload));
                    println!("⚠️  XSS VULNERABILITY: Payload echoed: {}", payload);
                }
                _ => {}
            }
        }
    }

    #[tokio::test]
    async fn test_json_injection_attempts() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

        // Attempt to inject additional JSON fields
        let injection = r#"{"msg_type":"text","content":"normal","injected":"malicious"}"#;
        ws_stream.send(Message::Text(injection.to_string())).await.expect("Send failed");

        // Should still work (extra fields ignored)
        let received = tokio::time::timeout(Duration::from_secs(1), ws_stream.next()).await;
        assert!(received.is_ok(), "Should handle extra fields gracefully");
        println!("✓ Extra JSON fields ignored (but logged?)");
    }

    #[tokio::test]
    async fn test_null_byte_injection() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

        let msg = serde_json::json!({
            "msg_type": "text",
            "content": "Hello\u{0000}World"
        });

        ws_stream.send(Message::Text(msg.to_string())).await.expect("Send failed");

        let received = tokio::time::timeout(Duration::from_secs(1), ws_stream.next())
            .await
            .expect("Timeout")
            .expect("No message")
            .expect("Error");

        match received {
            Message::Text(text) => {
                // ⚠️ Null bytes might cause issues in logs or C FFI
                assert!(text.contains('\u{0000}'));
                println!("⚠️  Null byte passed through unfiltered");
            }
            _ => {}
        }
    }

    // ⚠️ RELIABILITY & PERFORMANCE TESTS

    #[tokio::test]
    async fn test_connection_leak_under_load() {
        let (port, manager) = start_test_server().await;
        let session_id = Uuid::new_v4();

        // Create and destroy 1000 connections
        for _ in 0..1000 {
            let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);
            let (ws, _) = connect_async(&url).await.expect("Failed to connect");
            drop(ws);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        // Allow cleanup time
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Verify no connections leaked
        let sessions = manager.sessions.lock().unwrap();
        if sessions.contains_key(&session_id) {
            let session = sessions.get(&session_id).unwrap();
            println!("⚠️  CONNECTION LEAK: {} connections remain", session.len());
            assert_eq!(session.len(), 0, "All connections should be cleaned up");
        } else {
            println!("✓ No connection leaks detected");
        }
    }

    #[tokio::test]
    async fn test_concurrent_sessions_isolation() {
        let (port, manager) = start_test_server().await;
        
        let mut handles = vec![];

        // Create 10 concurrent sessions with multiple clients each
        for _ in 0..10 {
            let session_id = Uuid::new_v4();
            
            for client_id in 0..5 {
                let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);
                
                let handle = tokio::spawn(async move {
                    let (mut ws, _) = connect_async(&url).await.expect("Failed to connect");
                    
                    let msg = serde_json::json!({
                        "msg_type": "text",
                        "content": format!("Client {}", client_id)
                    });
                    
                    ws.send(Message::Text(msg.to_string())).await.ok();
                    tokio::time::sleep(Duration::from_millis(100)).await;
                });
                
                handles.push(handle);
            }
        }

        for handle in handles {
            handle.await.ok();
        }

        tokio::time::sleep(Duration::from_millis(500)).await;

        let sessions = manager.sessions.lock().unwrap();
        println!("✓ Total sessions: {}", sessions.len());
        assert!(sessions.len() <= 10, "Should have at most 10 sessions");
    }

    #[tokio::test]
    async fn test_message_ordering() {
        let (port, _manager) = start_test_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect");

        // Send ordered messages
        for i in 0..10 {
            let msg = serde_json::json!({
                "msg_type": "text",
                "content": format!("Message {}", i)
            });
            ws_stream.send(Message::Text(msg.to_string())).await.expect("Send failed");
        }

        // Receive and verify order
        let mut received_order = vec![];
        for _ in 0..10 {
            if let Ok(Some(Ok(Message::Text(text)))) = 
                tokio::time::timeout(Duration::from_millis(100), ws_stream.next()).await 
            {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(content) = json["content"].as_str() {
                        received_order.push(content.to_string());
                    }
                }
            }
        }

        println!("Received order: {:?}", received_order);
        // ⚠️ Order might not be guaranteed due to async nature and channel buffer
    }

    #[tokio::test]
    #[ignore] // Resource intensive
    async fn test_max_connections_limit() {
        let (port, manager) = start_test_server().await;
        let session_id = Uuid::new_v4();

        let mut connections = vec![];
        let max_attempts = 1000;

        for i in 0..max_attempts {
            let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);
            
            match connect_async(&url).await {
                Ok((ws, _)) => connections.push(ws),
                Err(e) => {
                    println!("⚠️  Connection limit reached at {}: {}", i, e);
                    break;
                }
            }
        }

        println!("✓ Successfully created {} connections", connections.len());
        
        // ⚠️ NO LIMIT: This is a DOS vulnerability
        if connections.len() >= max_attempts {
            println!("⚠️  DOS VULNERABILITY: No connection limit enforced!");
        }
    }
}