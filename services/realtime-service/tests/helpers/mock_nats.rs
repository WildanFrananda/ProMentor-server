use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

/// Mock NATS message
#[derive(Debug, Clone)]
pub struct MockNatsMessage {
    pub subject: String,
    pub payload: Vec<u8>,
}

/// Mock NATS server for testing
pub struct MockNatsServer {
    messages: Arc<Mutex<Vec<MockNatsMessage>>>,
    subscribers: Arc<Mutex<HashMap<String, Vec<tokio::sync::mpsc::Sender<MockNatsMessage>>>>>,
}

impl MockNatsServer {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            subscribers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Publish a message to a subject
    pub async fn publish(&self, subject: &str, payload: Vec<u8>) {
        let msg = MockNatsMessage {
            subject: subject.to_string(),
            payload,
        };

        // Store message
        self.messages.lock().await.push(msg.clone());

        // Notify subscribers
        let subscribers = self.subscribers.lock().await;
        if let Some(subs) = subscribers.get(subject) {
            for tx in subs {
                let _ = tx.send(msg.clone()).await;
            }
        }
    }

    /// Subscribe to a subject
    pub async fn subscribe(&self, subject: &str) -> tokio::sync::mpsc::Receiver<MockNatsMessage> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        let mut subscribers = self.subscribers.lock().await;
        subscribers.entry(subject.to_string())
            .or_insert_with(Vec::new)
            .push(tx);

        rx
    }

    /// Get all messages published
    pub async fn get_messages(&self) -> Vec<MockNatsMessage> {
        self.messages.lock().await.clone()
    }

    /// Clear all messages
    pub async fn clear(&self) {
        self.messages.lock().await.clear();
    }

    /// Get message count for a subject
    pub async fn message_count(&self, subject: &str) -> usize {
        self.messages.lock().await
            .iter()
            .filter(|m| m.subject == subject)
            .count()
    }
}

#[cfg(test)]
mod mock_nats_tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_mock_nats_publish_subscribe() {
        let server = MockNatsServer::new();
        let mut rx = server.subscribe("test.subject").await;

        server.publish("test.subject", b"test message".to_vec()).await;

        let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("Should receive message")
            .expect("Message should exist");

        assert_eq!(msg.subject, "test.subject");
        assert_eq!(msg.payload, b"test message");
    }

    #[tokio::test]
    async fn test_mock_nats_multiple_subscribers() {
        let server = MockNatsServer::new();
        let mut rx1 = server.subscribe("test").await;
        let mut rx2 = server.subscribe("test").await;

        server.publish("test", b"message".to_vec()).await;

        let msg1 = rx1.recv().await.unwrap();
        let msg2 = rx2.recv().await.unwrap();

        assert_eq!(msg1.payload, msg2.payload);
    }
}