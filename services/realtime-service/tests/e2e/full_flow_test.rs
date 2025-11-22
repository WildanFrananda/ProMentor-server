use actix_web::{web, App, HttpServer};
use realtime_service::api::ws_handler;
use realtime_service::services::session_manager::SessionManager;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;
use std::time::Duration;

#[cfg(test)]
mod e2e_tests {
    use super::*;

    async fn start_full_server() -> (u16, web::Data<SessionManager>) {
        let manager = web::Data::new(SessionManager::new());
        let manager_clone = manager.clone();

        // Start NATS listener in background
        tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager_clone.clone()
        ));

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
        
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        (port, manager)
    }

    #[tokio::test]
    async fn test_complete_chat_flow() {
        let (port, _manager) = start_full_server().await;
        let session_id = Uuid::new_v4();

        // User A connects
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);
        let (mut ws_a, _) = connect_async(&url).await.expect("User A failed to connect");

        // User B connects
        let (mut ws_b, _) = connect_async(&url).await.expect("User B failed to connect");

        tokio::time::sleep(Duration::from_millis(100)).await;

        // User A sends message
        let msg_a = serde_json::json!({
            "msg_type": "text",
            "content": "Hello from User A"
        });
        ws_a.send(Message::Text(msg_a.to_string())).await.expect("Send failed");

        // User B should receive
        let received_b = tokio::time::timeout(Duration::from_secs(1), ws_b.next())
            .await
            .expect("User B timeout")
            .expect("No message")
            .expect("Error");

        match received_b {
            Message::Text(text) => {
                let json: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(json["content"], "Hello from User A");
                println!("✓ End-to-end message delivery successful");
            }
            _ => panic!("Expected text message"),
        }

        // User B replies
        let msg_b = serde_json::json!({
            "msg_type": "text",
            "content": "Hello from User B"
        });
        ws_b.send(Message::Text(msg_b.to_string())).await.expect("Send failed");

        // User A should receive
        let received_a = tokio::time::timeout(Duration::from_secs(1), ws_a.next())
            .await
            .expect("User A timeout")
            .expect("No message")
            .expect("Error");

        match received_a {
            Message::Text(text) => {
                let json: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(json["content"], "Hello from User B");
                println!("✓ Bidirectional communication works");
            }
            _ => panic!("Expected text message"),
        }
    }

    #[tokio::test]
    async fn test_user_join_leave_flow() {
        let (port, manager) = start_full_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        // User 1 joins
        let (ws_1, _) = connect_async(&url).await.expect("Failed to connect");
        tokio::time::sleep(Duration::from_millis(100)).await;

        {
            let sessions = manager.sessions.lock().unwrap();
            let session = sessions.get(&session_id).unwrap();
            assert_eq!(session.len(), 1, "Should have 1 user");
        }

        // User 2 joins
        let (ws_2, _) = connect_async(&url).await.expect("Failed to connect");
        tokio::time::sleep(Duration::from_millis(100)).await;

        {
            let sessions = manager.sessions.lock().unwrap();
            let session = sessions.get(&session_id).unwrap();
            assert_eq!(session.len(), 2, "Should have 2 users");
        }

        // User 1 leaves
        drop(ws_1);
        tokio::time::sleep(Duration::from_millis(200)).await;

        {
            let sessions = manager.sessions.lock().unwrap();
            let session = sessions.get(&session_id).unwrap();
            assert_eq!(session.len(), 1, "Should have 1 user remaining");
        }

        // User 2 leaves
        drop(ws_2);
        tokio::time::sleep(Duration::from_millis(200)).await;

        {
            let sessions = manager.sessions.lock().unwrap();
            assert!(!sessions.contains_key(&session_id), "Session should be removed");
        }

        println!("✓ User join/leave flow works correctly");
    }

    #[tokio::test]
    async fn test_session_isolation() {
        let (port, _manager) = start_full_server().await;
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        // Connect to both sessions
        let url_a = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_a);
        let url_b = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_b);

        let (mut ws_a, _) = connect_async(&url_a).await.expect("Failed");
        let (mut ws_b, _) = connect_async(&url_b).await.expect("Failed");

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Send in session A
        let msg = serde_json::json!({
            "msg_type": "text",
            "content": "Session A only"
        });
        ws_a.send(Message::Text(msg.to_string())).await.expect("Send failed");

        // Session B should NOT receive
        let result = tokio::time::timeout(Duration::from_millis(300), ws_b.next()).await;
        assert!(result.is_err(), "Session B should not receive message from Session A");

        println!("✓ Session isolation confirmed");
    }

    #[tokio::test]
    async fn test_reconnection_after_disconnect() {
        let (port, _manager) = start_full_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        // First connection
        let (mut ws_1, _) = connect_async(&url).await.expect("Failed to connect");
        
        let msg = serde_json::json!({
            "msg_type": "text",
            "content": "First connection"
        });
        ws_1.send(Message::Text(msg.to_string())).await.expect("Send failed");

        // Disconnect
        drop(ws_1);
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Reconnect
        let (mut ws_2, _) = connect_async(&url).await.expect("Failed to reconnect");
        
        let msg = serde_json::json!({
            "msg_type": "text",
            "content": "After reconnection"
        });
        ws_2.send(Message::Text(msg.to_string())).await.expect("Send failed");

        let received = tokio::time::timeout(Duration::from_secs(1), ws_2.next())
            .await
            .expect("Timeout")
            .expect("No message")
            .expect("Error");

        println!("✓ Reconnection works: {:?}", received);
    }

    #[tokio::test]
    #[ignore] // Requires NATS
    async fn test_nats_to_websocket_flow() {
        use async_nats::Client;

        let (port, _manager) = start_full_server().await;
        let session_id = Uuid::new_v4();

        // Connect WebSocket client
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);
        let (mut ws, _) = connect_async(&url).await.expect("Failed to connect");

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Publish NATS event
        let nats_client = Client::connect("nats://localhost:4222")
            .await
            .expect("Failed to connect to NATS");

        let payload = serde_json::json!({
            "event_type": "session.created",
            "session_id": session_id
        });

        nats_client.publish("session.created", payload.to_string().into())
            .await
            .expect("Failed to publish");

        // WebSocket should receive
        let received = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .expect("Should receive NATS event")
            .expect("No message")
            .expect("Error");

        match received {
            Message::Text(text) => {
                let json: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(json["type"], "session.created");
                println!("✓ NATS → WebSocket flow works");
            }
            _ => panic!("Expected text message"),
        }
    }

    #[tokio::test]
    async fn test_multi_user_conversation() {
        let (port, _manager) = start_full_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        // Connect 5 users
        let mut clients = vec![];
        for i in 0..5 {
            let (ws, _) = connect_async(&url).await.expect("Failed to connect");
            clients.push(ws);
            println!("User {} connected", i + 1);
        }

        tokio::time::sleep(Duration::from_millis(200)).await;

        // User 0 sends message
        let msg = serde_json::json!({
            "msg_type": "text",
            "content": "Hello everyone!"
        });
        clients[0].send(Message::Text(msg.to_string())).await.expect("Send failed");

        // All other users should receive
        let mut received_count = 0;
        for (i, client) in clients.iter_mut().enumerate().skip(1) {
            if let Ok(Some(Ok(Message::Text(_)))) = 
                tokio::time::timeout(Duration::from_secs(1), client.next()).await 
            {
                received_count += 1;
                println!("User {} received message", i + 1);
            }
        }

        assert_eq!(received_count, 4, "All 4 other users should receive");
        println!("✓ Multi-user conversation works");
    }

    #[tokio::test]
    async fn test_graceful_shutdown_simulation() {
        let (port, manager) = start_full_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        // Connect clients
        let mut clients = vec![];
        for _ in 0..10 {
            let (ws, _) = connect_async(&url).await.expect("Failed");
            clients.push(ws);
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Simulate shutdown: close all connections
        for ws in clients {
            drop(ws);
        }

        tokio::time::sleep(Duration::from_millis(500)).await;

        // Verify cleanup
        let sessions = manager.sessions.lock().unwrap();
        assert!(!sessions.contains_key(&session_id), "All connections should be cleaned up");
        
        println!("✓ Graceful shutdown cleanup works");
    }

    #[tokio::test]
    async fn test_message_latency() {
        let (port, _manager) = start_full_server().await;
        let session_id = Uuid::new_v4();
        let url = format!("ws://127.0.0.1:{}/v1/ws/{}", port, session_id);

        let (mut ws_sender, _) = connect_async(&url).await.expect("Failed");
        let (mut ws_receiver, _) = connect_async(&url).await.expect("Failed");

        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut latencies = vec![];

        for _ in 0..100 {
            let start = std::time::Instant::now();

            let msg = serde_json::json!({
                "msg_type": "text",
                "content": "Latency test"
            });
            ws_sender.send(Message::Text(msg.to_string())).await.expect("Send failed");

            if let Ok(Some(Ok(_))) = tokio::time::timeout(Duration::from_secs(1), ws_receiver.next()).await {
                let latency = start.elapsed();
                latencies.push(latency);
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
        let max_latency = latencies.iter().max().unwrap();

        println!("Average latency: {:?}", avg_latency);
        println!("Max latency: {:?}", max_latency);

        assert!(avg_latency < Duration::from_millis(50), "Average latency too high");
        println!("✓ Message latency acceptable");
    }
}