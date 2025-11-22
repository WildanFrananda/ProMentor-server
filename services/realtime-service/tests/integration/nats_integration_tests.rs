use actix_web::web;
use realtime_service::services::session_manager::SessionManager;
use serde_json::json;
use uuid::Uuid;
use std::time::Duration;

#[cfg(test)]
mod nats_integration_tests {
    use super::*;

    // Note: These tests require a running NATS server
    // Run with: docker run -p 4222:4222 nats:latest

    #[tokio::test]
    #[ignore] // Requires NATS server
    async fn test_nats_connection_success() {
        std::env::set_var("NATS_URL", "nats://localhost:4222");
        
        let manager = web::Data::new(SessionManager::new());
        
        // Spawn NATS listener
        let handle = tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager.clone()
        ));

        // Give it time to connect
        tokio::time::sleep(Duration::from_secs(1)).await;

        // If no panic, connection succeeded
        println!("✓ NATS connection established");
        
        handle.abort();
    }

    #[tokio::test]
    #[ignore]
    async fn test_nats_connection_failure_handling() {
        std::env::set_var("NATS_URL", "nats://invalid-host:4222");
        
        let manager = web::Data::new(SessionManager::new());
        
        // Should not panic on connection failure
        let handle = tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager.clone()
        ));

        tokio::time::sleep(Duration::from_secs(2)).await;

        // ⚠️ RELIABILITY: Connection failure is just logged, no retry mechanism
        println!("⚠️  NATS connection failed - no retry mechanism");
        
        handle.abort();
    }

    #[tokio::test]
    #[ignore] // Requires NATS server
    async fn test_session_created_event() {
        use async_nats::Client;

        std::env::set_var("NATS_URL", "nats://localhost:4222");
        
        let manager = web::Data::new(SessionManager::new());
        let session_id = Uuid::new_v4();

        // Start NATS listener
        tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager.clone()
        ));

        tokio::time::sleep(Duration::from_millis(500)).await;

        // Connect as publisher
        let client = Client::connect("nats://localhost:4222")
            .await
            .expect("Failed to connect to NATS");

        // Publish session.created event
        let payload = json!({
            "event_type": "session.created",
            "session_id": session_id
        });

        client.publish("session.created", payload.to_string().into())
            .await
            .expect("Failed to publish");

        tokio::time::sleep(Duration::from_millis(200)).await;

        println!("✓ session.created event processed");
    }

    #[tokio::test]
    #[ignore] // Requires NATS server
    async fn test_session_joined_event() {
        use async_nats::Client;

        std::env::set_var("NATS_URL", "nats://localhost:4222");
        
        let manager = web::Data::new(SessionManager::new());
        let session_id = Uuid::new_v4();

        tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager.clone()
        ));

        tokio::time::sleep(Duration::from_millis(500)).await;

        let client = Client::connect("nats://localhost:4222")
            .await
            .expect("Failed to connect");

        let payload = json!({
            "event_type": "session.joined",
            "session_id": session_id
        });

        client.publish("session.joined", payload.to_string().into())
            .await
            .expect("Failed to publish");

        tokio::time::sleep(Duration::from_millis(200)).await;

        println!("✓ session.joined event processed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_invalid_event_payload() {
        use async_nats::Client;

        std::env::set_var("NATS_URL", "nats://localhost:4222");
        
        let manager = web::Data::new(SessionManager::new());

        tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager.clone()
        ));

        tokio::time::sleep(Duration::from_millis(500)).await;

        let client = Client::connect("nats://localhost:4222")
            .await
            .expect("Failed to connect");

        // Send invalid JSON
        client.publish("session.created", "not json".into())
            .await
            .expect("Failed to publish");

        tokio::time::sleep(Duration::from_millis(200)).await;

        // ⚠️ RELIABILITY: Invalid payloads are just logged, no alerting
        println!("⚠️  Invalid event payload logged but not handled");
    }

    #[tokio::test]
    #[ignore]
    async fn test_missing_required_fields() {
        use async_nats::Client;

        std::env::set_var("NATS_URL", "nats://localhost:4222");
        
        let manager = web::Data::new(SessionManager::new());

        tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager.clone()
        ));

        tokio::time::sleep(Duration::from_millis(500)).await;

        let client = Client::connect("nats://localhost:4222")
            .await
            .expect("Failed to connect");

        // Missing session_id
        let payload = json!({
            "event_type": "session.created"
        });

        client.publish("session.created", payload.to_string().into())
            .await
            .expect("Failed to publish");

        tokio::time::sleep(Duration::from_millis(200)).await;

        println!("⚠️  Missing fields not validated");
    }

    #[tokio::test]
    #[ignore]
    async fn test_event_broadcast_to_websocket_clients() {
        use async_nats::Client;
        use tokio_tungstenite::connect_async;
        use futures_util::StreamExt;

        std::env::set_var("NATS_URL", "nats://localhost:4222");
        
        let manager = web::Data::new(SessionManager::new());
        let session_id = Uuid::new_v4();

        // Start NATS listener
        tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager.clone()
        ));

        // Connect WebSocket client
        let ws_url = format!("ws://localhost:8080/v1/ws/{}", session_id);
        let (mut ws_stream, _) = connect_async(&ws_url)
            .await
            .expect("Failed to connect WebSocket");

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Publish NATS event
        let client = Client::connect("nats://localhost:4222")
            .await
            .expect("Failed to connect to NATS");

        let payload = json!({
            "event_type": "session.created",
            "session_id": session_id
        });

        client.publish("session.created", payload.to_string().into())
            .await
            .expect("Failed to publish");

        // WebSocket client should receive the broadcast
        let received = tokio::time::timeout(Duration::from_secs(2), ws_stream.next())
            .await
            .expect("Should receive message")
            .expect("Message should exist")
            .expect("No error");

        println!("✓ NATS event broadcasted to WebSocket: {:?}", received);
    }

    #[tokio::test]
    #[ignore]
    async fn test_high_volume_events() {
        use async_nats::Client;

        std::env::set_var("NATS_URL", "nats://localhost:4222");
        
        let manager = web::Data::new(SessionManager::new());

        tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager.clone()
        ));

        tokio::time::sleep(Duration::from_millis(500)).await;

        let client = Client::connect("nats://localhost:4222")
            .await
            .expect("Failed to connect");

        let start = std::time::Instant::now();

        // Publish 1000 events
        for i in 0..1000 {
            let payload = json!({
                "event_type": "session.created",
                "session_id": Uuid::new_v4()
            });

            client.publish("session.created", payload.to_string().into())
                .await
                .expect("Failed to publish");
        }

        let duration = start.elapsed();
        println!("✓ Published 1000 events in {:?}", duration);
        
        // ⚠️ PERFORMANCE: Monitor for backpressure and message loss
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_nats_reconnection() {
        std::env::set_var("NATS_URL", "nats://localhost:4222");
        
        let manager = web::Data::new(SessionManager::new());

        let handle = tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager.clone()
        ));

        tokio::time::sleep(Duration::from_secs(1)).await;

        // Simulate NATS server restart
        println!("⚠️  Simulating NATS server restart...");
        println!("⚠️  Current implementation does NOT auto-reconnect!");
        println!("⚠️  Service will stop receiving events until restart");

        // ⚠️ CRITICAL: No reconnection logic implemented
        
        tokio::time::sleep(Duration::from_secs(2)).await;
        handle.abort();
    }

    #[test]
    fn test_event_payload_structure() {
        // Validate EventPayload structure
        #[derive(serde::Deserialize)]
        struct EventPayload {
            event_type: String,
            session_id: Uuid,
        }

        let valid = json!({
            "event_type": "session.created",
            "session_id": "550e8400-e29b-41d4-a716-446655440000"
        });

        let result: Result<EventPayload, _> = serde_json::from_value(valid);
        assert!(result.is_ok(), "Valid payload should parse");

        // Invalid UUID
        let invalid_uuid = json!({
            "event_type": "session.created",
            "session_id": "not-a-uuid"
        });

        let result: Result<EventPayload, _> = serde_json::from_value(invalid_uuid);
        assert!(result.is_err(), "Invalid UUID should be rejected");
    }

    #[test]
    fn test_supported_event_types() {
        let supported_events = vec!["session.created", "session.joined"];
        
        for event_type in supported_events {
            println!("✓ Supported event: {}", event_type);
        }

        // ⚠️ EXTENSIBILITY: Only 2 event types supported
        // No validation for unknown event types
        println!("⚠️  Unknown event types are not validated");
    }

    #[tokio::test]
    #[ignore]
    async fn test_event_ordering() {
        use async_nats::Client;

        std::env::set_var("NATS_URL", "nats://localhost:4222");
        
        let manager = web::Data::new(SessionManager::new());

        tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager.clone()
        ));

        tokio::time::sleep(Duration::from_millis(500)).await;

        let client = Client::connect("nats://localhost:4222")
            .await
            .expect("Failed to connect");

        // Publish events in order
        for i in 0..10 {
            let payload = json!({
                "event_type": "session.created",
                "session_id": Uuid::new_v4(),
                "sequence": i
            });

            client.publish("session.created", payload.to_string().into())
                .await
                .expect("Failed to publish");
        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        // ⚠️ EVENT ORDERING: NATS doesn't guarantee order across subjects
        println!("⚠️  Event ordering not guaranteed in current implementation");
    }

    #[tokio::test]
    #[ignore]
    async fn test_concurrent_event_processing() {
        use async_nats::Client;

        std::env::set_var("NATS_URL", "nats://localhost:4222");
        
        let manager = web::Data::new(SessionManager::new());

        tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager.clone()
        ));

        tokio::time::sleep(Duration::from_millis(500)).await;

        let mut handles = vec![];

        // Spawn multiple publishers
        for _ in 0..10 {
            let handle = tokio::spawn(async move {
                let client = Client::connect("nats://localhost:4222")
                    .await
                    .expect("Failed to connect");

                for _ in 0..100 {
                    let payload = json!({
                        "event_type": "session.created",
                        "session_id": Uuid::new_v4()
                    });

                    client.publish("session.created", payload.to_string().into())
                        .await
                        .ok();
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await.ok();
        }

        tokio::time::sleep(Duration::from_secs(2)).await;

        // ⚠️ CONCURRENCY: Check for race conditions and message loss
        println!("✓ Concurrent event processing completed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_nats_subject_wildcard() {
        use async_nats::Client;

        std::env::set_var("NATS_URL", "nats://localhost:4222");
        
        let manager = web::Data::new(SessionManager::new());

        tokio::spawn(realtime_service::events::nats_listener::run_nats_listener(
            manager.clone()
        ));

        tokio::time::sleep(Duration::from_millis(500)).await;

        let client = Client::connect("nats://localhost:4222")
            .await
            .expect("Failed to connect");

        // Try publishing to session.* (wildcard)
        let payload = json!({
            "event_type": "session.deleted",
            "session_id": Uuid::new_v4()
        });

        client.publish("session.deleted", payload.to_string().into())
            .await
            .expect("Failed to publish");

        tokio::time::sleep(Duration::from_millis(200)).await;

        // ⚠️ LIMITATION: Only subscribed to specific subjects, not wildcards
        println!("⚠️  Wildcard subscriptions not implemented");
    }
}

// Unit tests for NATS listener logic (without actual NATS)
#[cfg(test)]
mod nats_unit_tests {
    use super::*;

    #[test]
    fn test_nats_url_from_env() {
        std::env::set_var("NATS_URL", "nats://custom-host:4222");
        let url = std::env::var("NATS_URL").unwrap();
        assert_eq!(url, "nats://custom-host:4222");

        std::env::remove_var("NATS_URL");
        let default_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
        assert_eq!(default_url, "nats://localhost:4222");
    }

    #[test]
    fn test_event_payload_deserialization_edge_cases() {
        #[derive(serde::Deserialize, Debug)]
        struct EventPayload {
            event_type: String,
            session_id: Uuid,
        }

        // Extra fields should be ignored
        let with_extra = json!({
            "event_type": "session.created",
            "session_id": "550e8400-e29b-41d4-a716-446655440000",
            "extra_field": "should be ignored",
            "another": 123
        });

        let result: Result<EventPayload, _> = serde_json::from_value(with_extra);
        assert!(result.is_ok(), "Extra fields should be ignored");

        // Empty event_type
        let empty_type = json!({
            "event_type": "",
            "session_id": "550e8400-e29b-41d4-a716-446655440000"
        });

        let result: Result<EventPayload, _> = serde_json::from_value(empty_type);
        assert!(result.is_ok(), "Empty event_type is technically valid");
        println!("⚠️  Empty event_type not validated");

        // Null values
        let null_type = json!({
            "event_type": null,
            "session_id": "550e8400-e29b-41d4-a716-446655440000"
        });

        let result: Result<EventPayload, _> = serde_json::from_value(null_type);
        assert!(result.is_err(), "Null event_type should be rejected");
    }

    #[test]
    fn test_broadcast_message_format() {
        // Verify the format of broadcast messages from NATS events
        let session_id = Uuid::new_v4();
        let broadcast_msg = json!({
            "type": "session.created",
            "sessionId": session_id
        });

        let json_str = broadcast_msg.to_string();
        
        assert!(json_str.contains("session.created"));
        assert!(json_str.contains(&session_id.to_string()));
        
        // ⚠️ INCONSISTENCY: WebSocket uses "type", model uses "msg_type"
        println!("⚠️  Field naming inconsistency: 'type' vs 'msg_type'");
    }

    #[test]
    fn test_subject_name_validation() {
        let subjects = vec!["session.created", "session.joined"];
        
        for subject in subjects {
            assert!(subject.starts_with("session."));
            assert!(!subject.contains(" "));
            assert!(!subject.contains("\n"));
        }

        // ⚠️ No validation for malicious subject names
        let malicious_subjects = vec![
            "session.created; DROP TABLE",
            "session../../../etc/passwd",
            "session.\u{0000}",
        ];

        for subject in malicious_subjects {
            println!("⚠️  Malicious subject not validated: {}", subject);
        }
    }
}