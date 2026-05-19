use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub node_id: String,
    pub name: String,
    pub paired_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub device_id: Option<String>,
    #[serde(default)]
    pub deleted: bool,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub content: Option<String>,
    pub due_date: Option<i64>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub device_id: Option<String>,
    #[serde(default)]
    pub deleted: bool,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransfer {
    pub id: String,
    pub filename: String,
    pub size_bytes: i64,
    pub blake3_hash: Option<String>,
    pub status: String,
    pub source_node: String,
    pub target_node: String,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}
