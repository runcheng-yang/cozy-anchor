use serde::{Deserialize, Serialize};
use crate::models::{Message, Todo};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMessage {
    Pull { tables: SyncTables },
    Data { tables: DataTables },
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTables {
    pub messages: i64,
    pub todos: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataTables {
    pub messages: Vec<Message>,
    pub todos: Vec<Todo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileProtocol {
    Offer { transfer_id: String, filename: String, size: i64 },
    Accept { transfer_id: String },
    Decline { transfer_id: String },
    Chunk { transfer_id: String, offset: i64, data: String },
    Complete { transfer_id: String },
}
