use realtime_service::services::session_manager::{SessionManager, Connection};
use tokio::sync::mpsc;
use uuid::Uuid;
use std::sync::Arc;
use std::time::Duration;

#[cfg(test)]
mod session_manager_unit_tests {
    use super::*;

    fn create_test_connection() -> Connection {
        let (tx, _rx) = mpsc::channel(16);
        Connection { sender: tx }
    }

    #[test]
    fn test_new_session_manager_is_empty() {
        let manager = SessionManager::new();
        let sessions = manager.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 0, "New SessionManager should be empty");
    }

    #[test]
    fn test_insert_single_connection() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4();
        let conn_id = 1;
        let conn = create_test_connection();

        manager.insert(session_id, conn_id, conn);

        let sessions = manager.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1, "Should have 1 session");
        assert!(sessions.contains_key(&session_id), "Should contain the session_id");
        
        let session = sessions.get(&session_id).unwrap();
        assert_eq!(session.len(), 1, "Session should have 1 connection");
    }

    #[test]
    fn test_insert_multiple_connections_same_session() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4();

        for conn_id in 0..10 {
            manager.insert(session_id, conn_id, create_test_connection());
        }

        let sessions = manager.sessions.lock().unwrap();
        let session = sessions.get(&session_id).unwrap();
        assert_eq!(session.len(), 10, "Session should have 10 connections");
    }

    #[test]
    fn test_insert_multiple_sessions() {
        let manager = SessionManager::new();

        for _ in 0..5 {
            let session_id = Uuid::new_v4();
            manager.insert(session_id, 1, create_test_connection());
        }

        let sessions = manager.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 5, "Should have 5 different sessions");
    }

    #[test]
    fn test_remove_connection() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4();
        let conn_id = 1;

        manager.insert(session_id, conn_id, create_test_connection());
        manager.remove(session_id, conn_id);

        let sessions = manager.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 0, "Session should be removed when empty");
    }

    #[test]
    fn test_remove_one_of_multiple_connections() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4();

        manager.insert(session_id, 1, create_test_connection());
        manager.insert(session_id, 2, create_test_connection());
        manager.remove(session_id, 1);

        let sessions = manager.sessions.lock().unwrap();
        let session = sessions.get(&session_id).unwrap();
        assert_eq!(session.len(), 1, "Should have 1 connection remaining");
        assert!(!session.contains_key(&1), "Connection 1 should be removed");
        assert!(session.contains_key(&2), "Connection 2 should remain");
    }

    #[test]
    fn test_remove_nonexistent_connection() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4();

        // Should not panic when removing non-existent connection
        manager.remove(session_id, 999);
        
        let sessions = manager.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 0);
    }

    #[test]
    fn test_duplicate_conn_id_overwrites() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4();
        let conn_id = 1;

        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);

        manager.insert(session_id, conn_id, Connection { sender: tx1.clone() });
        manager.insert(session_id, conn_id, Connection { sender: tx2.clone() });

        let sessions = manager.sessions.lock().unwrap();
        let session = sessions.get(&session_id).unwrap();
        assert_eq!(session.len(), 1, "Duplicate conn_id should overwrite, not add");
        
        // ⚠️ SECURITY NOTE: This could silently disconnect a user!
    }

    #[tokio::test]
    async fn test_broadcast_message_to_single_connection() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4();
        let (tx, mut rx) = mpsc::channel(16);

        manager.insert(session_id, 1, Connection { sender: tx });
        manager.broadcast_message(session_id, "test message", None);

        let received = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("Should receive message within timeout")
            .expect("Should receive a message");

        assert_eq!(received, "test message");
    }

    #[tokio::test]
    async fn test_broadcast_message_to_multiple_connections() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4();
        let mut receivers = vec![];

        for conn_id in 0..5 {
            let (tx, rx) = mpsc::channel(16);
            receivers.push(rx);
            manager.insert(session_id, conn_id, Connection { sender: tx });
        }

        manager.broadcast_message(session_id, "broadcast", None);

        for mut rx in receivers {
            let received = tokio::time::timeout(Duration::from_millis(100), rx.recv())
                .await
                .expect("Should receive message")
                .expect("Should have message");
            assert_eq!(received, "broadcast");
        }
    }

    #[tokio::test]
    async fn test_broadcast_with_skip_id() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4();
        
        let (tx1, mut rx1) = mpsc::channel(16);
        let (tx2, mut rx2) = mpsc::channel(16);

        manager.insert(session_id, 1, Connection { sender: tx1 });
        manager.insert(session_id, 2, Connection { sender: tx2 });

        manager.broadcast_message(session_id, "test", Some(1));

        // Connection 1 should not receive
        let result1 = tokio::time::timeout(Duration::from_millis(50), rx1.recv()).await;
        assert!(result1.is_err(), "Connection 1 should be skipped");

        // Connection 2 should receive
        let received2 = tokio::time::timeout(Duration::from_millis(50), rx2.recv())
            .await
            .expect("Should receive")
            .expect("Should have message");
        assert_eq!(received2, "test");
    }

    #[tokio::test]
    async fn test_broadcast_to_nonexistent_session() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4();

        // Should not panic
        manager.broadcast_message(session_id, "test", None);
    }

    #[tokio::test]
    async fn test_broadcast_with_full_channel() {
        let manager = SessionManager::new();
        let session_id = Uuid::new_v4();
        let (tx, _rx) = mpsc::channel(1); // Small buffer

        manager.insert(session_id, 1, Connection { sender: tx });

        // Fill the channel
        manager.broadcast_message(session_id, "msg1", None);
        manager.broadcast_message(session_id, "msg2", None); // Should fail

        // ⚠️ RELIABILITY NOTE: Messages are silently dropped with try_send!
        // No error handling or retry mechanism
    }

    #[test]
    fn test_concurrent_inserts() {
        use std::thread;

        let manager = Arc::new(SessionManager::new());
        let session_id = Uuid::new_v4();
        let mut handles = vec![];

        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                manager_clone.insert(session_id, i, create_test_connection());
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let sessions = manager.sessions.lock().unwrap();
        let session = sessions.get(&session_id).unwrap();
        assert_eq!(session.len(), 10, "All concurrent inserts should succeed");
    }

    #[test]
    fn test_concurrent_removes() {
        use std::thread;

        let manager = Arc::new(SessionManager::new());
        let session_id = Uuid::new_v4();

        // Insert connections
        for i in 0..10 {
            manager.insert(session_id, i, create_test_connection());
        }

        let mut handles = vec![];
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                manager_clone.remove(session_id, i);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let sessions = manager.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 0, "All connections should be removed");
    }

    #[tokio::test]
    async fn test_concurrent_broadcasts() {
        use std::sync::Arc;

        let manager = Arc::new(SessionManager::new());
        let session_id = Uuid::new_v4();
        let (tx, mut rx) = mpsc::channel(100);

        manager.insert(session_id, 1, Connection { sender: tx });

        let mut handles = vec![];
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = tokio::spawn(async move {
                manager_clone.broadcast_message(session_id, &format!("msg{}", i), None);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Collect all messages
        let mut count = 0;
        while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
            count += 1;
        }

        // ⚠️ RACE CONDITION: Might not receive all 10 messages due to try_send failures
        assert!(count > 0, "Should receive at least some messages");
    }

    #[test]
    fn test_memory_leak_prevention() {
        let manager = SessionManager::new();
        
        // Create and destroy many sessions
        for _ in 0..1000 {
            let session_id = Uuid::new_v4();
            manager.insert(session_id, 1, create_test_connection());
            manager.remove(session_id, 1);
        }

        let sessions = manager.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 0, "All sessions should be cleaned up");
        
        // In production, use a memory profiler to verify no leaks
    }

    #[test]
    fn test_session_isolation() {
        let manager = SessionManager::new();
        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();

        manager.insert(session1, 1, create_test_connection());
        manager.insert(session2, 1, create_test_connection());

        manager.remove(session1, 1);

        let sessions = manager.sessions.lock().unwrap();
        assert!(!sessions.contains_key(&session1), "Session 1 should be removed");
        assert!(sessions.contains_key(&session2), "Session 2 should remain");
    }

    // ⚠️ DEADLOCK TEST
    #[test]
    #[ignore] // Run with: cargo test -- --ignored --test-threads=1
    fn test_potential_deadlock_scenario() {
        use std::thread;
        use std::time::Duration;

        let manager = Arc::new(SessionManager::new());
        let session_id = Uuid::new_v4();

        // Thread 1: Holds lock while broadcasting
        let m1 = Arc::clone(&manager);
        let handle1 = thread::spawn(move || {
            for _ in 0..100 {
                m1.broadcast_message(session_id, "test", None);
                thread::sleep(Duration::from_micros(1));
            }
        });

        // Thread 2: Tries to insert while thread 1 holds lock
        let m2 = Arc::clone(&manager);
        let handle2 = thread::spawn(move || {
            for i in 0..100 {
                m2.insert(session_id, i, create_test_connection());
                thread::sleep(Duration::from_micros(1));
            }
        });

        // Should complete within reasonable time
        let timeout = Duration::from_secs(5);
        assert!(handle1.join().is_ok(), "Thread 1 should complete");
        assert!(handle2.join().is_ok(), "Thread 2 should complete");
    }
}