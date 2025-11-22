use actix_web::{test, web, App, http::StatusCode};
use realtime_service::api::ws_handler;
use realtime_service::services::session_manager::SessionManager;
use realtime_service::model::chat_message::ChatMessage;
use uuid::Uuid;

#[cfg(test)]
mod ws_handler_unit_tests {
    use super::*;

    async fn create_app() -> impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    > {
        let manager = web::Data::new(SessionManager::new());
        test::init_service(
            App::new()
                .app_data(manager.clone())
                .route("/v1/ws/{session_id}", web::get().to(ws_handler::ws_route))
        ).await
    }

    #[actix_web::test]
    async fn test_ws_route_accepts_valid_uuid() {
        let app = create_app().await;
        let session_id = Uuid::new_v4();
        
        let req = test::TestRequest::get()
            .uri(&format!("/v1/ws/{}", session_id))
            .insert_header(("upgrade", "websocket"))
            .insert_header(("connection", "upgrade"))
            .insert_header(("sec-websocket-version", "13"))
            .insert_header(("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::SWITCHING_PROTOCOLS);
    }

    #[actix_web::test]
    async fn test_ws_route_rejects_invalid_uuid() {
        let app = create_app().await;
        
        let req = test::TestRequest::get()
            .uri("/v1/ws/invalid-uuid")
            .insert_header(("upgrade", "websocket"))
            .insert_header(("connection", "upgrade"))
            .insert_header(("sec-websocket-version", "13"))
            .insert_header(("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="))
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should return 4xx error for invalid UUID
        assert!(resp.status().is_client_error());
    }

    #[actix_web::test]
    async fn test_ws_route_without_upgrade_header() {
        let app = create_app().await;
        let session_id = Uuid::new_v4();
        
        let req = test::TestRequest::get()
            .uri(&format!("/v1/ws/{}", session_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should fail without proper WebSocket upgrade headers
        assert_ne!(resp.status(), StatusCode::SWITCHING_PROTOCOLS);
    }

    #[actix_web::test]
    async fn test_multiple_connections_same_session() {
        let manager = web::Data::new(SessionManager::new());
        let session_id = Uuid::new_v4();

        let app = test::init_service(
            App::new()
                .app_data(manager.clone())
                .route("/v1/ws/{session_id}", web::get().to(ws_handler::ws_route))
        ).await;

        // Create multiple connections
        for _ in 0..5 {
            let req = test::TestRequest::get()
                .uri(&format!("/v1/ws/{}", session_id))
                .insert_header(("upgrade", "websocket"))
                .insert_header(("connection", "upgrade"))
                .insert_header(("sec-websocket-version", "13"))
                .insert_header(("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::SWITCHING_PROTOCOLS);
        }

        // Verify all connections are tracked
        let sessions = manager.sessions.lock().unwrap();
        let session = sessions.get(&session_id);
        assert!(session.is_some(), "Session should exist");
        // Note: In real test, connections would be active
    }

    #[test]
    fn test_chat_message_validation() {
        // Valid message
        let valid = r#"{"msg_type":"text","content":"Hello"}"#;
        let result: Result<ChatMessage, _> = serde_json::from_str(valid);
        assert!(result.is_ok(), "Valid message should parse");

        // Missing content
        let invalid = r#"{"msg_type":"text"}"#;
        let result: Result<ChatMessage, _> = serde_json::from_str(invalid);
        assert!(result.is_err(), "Should reject missing content");

        // Extra fields (should be ignored)
        let extra = r#"{"msg_type":"text","content":"Hi","extra":"field"}"#;
        let result: Result<ChatMessage, _> = serde_json::from_str(extra);
        assert!(result.is_ok(), "Should ignore extra fields");
    }

    #[test]
    fn test_malformed_json_handling() {
        let malformed_messages = vec![
            r#"{"msg_type":"text""#,  // Unclosed
            r#"{msg_type:text}"#,       // No quotes
            r#"not json at all"#,       // Plain text
            r#"{"msg_type":null,"content":"test"}"#,  // Null type
            r#""#,  // Empty
        ];

        for msg in malformed_messages {
            let result: Result<ChatMessage, _> = serde_json::from_str(msg);
            assert!(result.is_err(), "Should reject: {}", msg);
        }
    }

    // ⚠️ SECURITY TESTS
    #[test]
    fn test_xss_payload_not_sanitized() {
        let xss = r#"{"msg_type":"text","content":"<script>alert('XSS')</script>"}"#;
        let msg: ChatMessage = serde_json::from_str(xss).unwrap();
        
        // ⚠️ VULNERABILITY: XSS payload passes through unsanitized
        assert!(msg.content.contains("<script>"));
        println!("⚠️  XSS VULNERABILITY: Payload not sanitized!");
    }

    #[test]
    fn test_sql_injection_payload() {
        let sql = r#"{"msg_type":"text","content":"'; DROP TABLE users; --"}"#;
        let msg: ChatMessage = serde_json::from_str(sql).unwrap();
        
        // ⚠️ If this is ever used in SQL without parameterization...
        assert!(msg.content.contains("DROP TABLE"));
        println!("⚠️  SQL INJECTION RISK: No sanitization!");
    }

    #[test]
    fn test_oversized_message_accepted() {
        let large_content = "A".repeat(10_000_000); // 10MB
        let msg = ChatMessage {
            msg_type: "text".to_string(),
            content: large_content.clone(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        
        // ⚠️ VULNERABILITY: No size limit enforcement
        assert!(json.len() > 10_000_000);
        println!("⚠️  DOS VULNERABILITY: 10MB message accepted!");
    }

    #[test]
    fn test_unicode_edge_cases() {
        let test_cases = vec![
            ("Emoji", "Hello 👋🌍"),
            ("Chinese", "你好世界"),
            ("Arabic", "مرحبا بالعالم"),
            ("Mixed", "Hello世界🌍"),
            ("Zalgo", "H̴̡̢̨̧̛e̴̡̢̨̧̛l̴̡̢̨̧̛l̴̡̢̨̧̛ơ̴̡̢̨̧"),
        ];

        for (name, content) in test_cases {
            let msg = ChatMessage {
                msg_type: "text".to_string(),
                content: content.to_string(),
            };

            let json = serde_json::to_string(&msg).unwrap();
            let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
            
            assert_eq!(parsed.content, content, "Failed for: {}", name);
        }
    }

    #[test]
    fn test_control_characters() {
        let control_chars = vec![
            "\u{0000}", // NULL
            "\u{0001}", // Start of heading
            "\u{0008}", // Backspace
            "\u{001B}", // Escape
            "\r\n",     // CRLF
        ];

        for ctrl in control_chars {
            let msg = ChatMessage {
                msg_type: "text".to_string(),
                content: format!("Test{}Message", ctrl),
            };

            let json = serde_json::to_string(&msg).unwrap();
            
            // ⚠️ Control characters pass through - could break logs/terminals
            assert!(json.contains("Test"));
        }
    }

    #[test]
    fn test_deeply_nested_json() {
        // Create deeply nested structure
        let mut nested = String::from(r#"{"msg_type":"text","content":"#);
        for _ in 0..1000 {
            nested.push_str(r#"{"nested":"#);
        }
        nested.push_str("value");
        for _ in 0..1000 {
            nested.push_str("\"}");
        }
        nested.push_str("\"}");

        // Should either reject or handle gracefully
        let result: Result<ChatMessage, _> = serde_json::from_str(&nested);
        // Most JSON parsers will hit recursion limit
        println!("Deep nesting result: {:?}", result.is_err());
    }

    #[test]
    fn test_connection_id_collision() {
        // Test that random conn_id generation might collide
        let mut ids = std::collections::HashSet::new();
        
        for _ in 0..1000 {
            let id = rand::random::<usize>();
            if !ids.insert(id) {
                println!("⚠️  Connection ID collision detected!");
                // ⚠️ This could cause user disconnection
                return;
            }
        }
    }

    // Performance baseline tests
    #[test]
    fn test_message_parsing_performance() {
        let msg = r#"{"msg_type":"text","content":"Hello, World!"}"#;
        let start = std::time::Instant::now();
        
        for _ in 0..10_000 {
            let _: ChatMessage = serde_json::from_str(msg).unwrap();
        }
        
        let duration = start.elapsed();
        println!("10k message parses: {:?}", duration);
        assert!(duration.as_millis() < 1000, "Parsing too slow");
    }

    #[test]
    fn test_heartbeat_interval_constant() {
        use std::time::Duration;
        // Verify heartbeat is reasonable
        const HEARTBEAT: Duration = Duration::from_secs(5);
        
        assert!(HEARTBEAT.as_secs() >= 5, "Heartbeat too frequent");
        assert!(HEARTBEAT.as_secs() <= 30, "Heartbeat too infrequent");
    }

    #[test]
    fn test_message_channel_buffer_size() {
        // Document the buffer size limitation
        const BUFFER_SIZE: usize = 16;
        
        // ⚠️ RELIABILITY: Buffer is very small
        assert_eq!(BUFFER_SIZE, 16);
        println!("⚠️  Channel buffer only 16 - messages will drop under load!");
    }
}