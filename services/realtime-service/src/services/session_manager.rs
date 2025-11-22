use crate::auth::jwt::Claims;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct Connection {
    pub sender: mpsc::Sender<String>,
    pub user_info: Claims
}

pub struct SessionManager {
    sessions: Mutex<HashMap<Uuid, HashMap<usize, Connection>>>
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager {
            sessions: Mutex::new(HashMap::new())
        }
    }

    pub fn insert(&self, session_id: Uuid, conn_id: usize, conn: Connection) {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.entry(session_id).or_insert_with(HashMap::new);

        println!("👤 User '{}' ({}) joined session {}. Total connections: {}", 
            conn.user_info.name, conn.user_info.sub, session_id, session.len() + 1);
        
        session.insert(conn_id, conn);
    }

    pub fn remove(&self, session_id: Uuid, conn_id: usize) {
        let mut sessions = self.sessions.lock().unwrap();

        if let Some(session) = sessions.get_mut(&session_id) {
            session.remove(&conn_id);
            println!("👋 Connection {} removed from session {}", conn_id, session_id);

            if session.is_empty() {
                sessions.remove(&session_id);
                println!("🗑️  Session {} empty and removed", session_id);
            }
        }
    }

    pub fn broadcast_message(&self, session_id: Uuid, message: &str, skip_id: Option<usize>) {
        let sessions = self.sessions.lock().unwrap();
        
        if let Some(session) = sessions.get(&session_id) {
            println!("📡 Broadcasting to session {} (skip_id: {:?}). Total recipients: {}", 
                session_id, skip_id, session.len());

            let mut sent_count = 0;
            let mut failed_count = 0;

            for (id, conn) in session {
                if skip_id.is_some() && skip_id.unwrap() == *id {
                    println!("⏭️  Skipping conn_id={} (sender)", id);
                    continue;
                }

                println!("📤 Attempting to send to conn_id={} ({})", id, conn.user_info.name);
                
                match conn.sender.try_send(message.to_string()) {
                    Ok(_) => {
                        sent_count += 1;
                        println!("✅ Message sent to conn_id={}", id);
                    },
                    Err(e) => {
                        failed_count += 1;
                        eprintln!("❌ Failed to send message to connection {} ({}): {:?}", 
                            id, conn.user_info.name, e);
                    }
                }
            }

            println!("📊 Broadcast summary: {} sent, {} failed", sent_count, failed_count);
        } else {
            eprintln!("❌ Session {} not found for broadcasting", session_id);
        }
    }

    pub fn get_user_info(&self, session_id: Uuid, conn_id: usize) -> Option<Claims> {
        let sessions = self.sessions.lock().unwrap();
        return sessions.get(&session_id)
            .and_then(|session| session.get(&conn_id))
            .map(|conn| conn.user_info.clone());
    }
}