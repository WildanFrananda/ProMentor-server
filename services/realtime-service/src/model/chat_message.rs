use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMessage {
    pub content: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BroadcastMessage {
    pub r#type: String,
    pub sender: SenderInfo,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SenderInfo {
    pub id: Uuid,
    pub name: String,
}