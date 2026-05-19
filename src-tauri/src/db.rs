use rusqlite::{Connection, Result, params};
use std::path::PathBuf;
use crate::models::*;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn new(app_dir: PathBuf) -> Result<Self> {
        let db_path = app_dir.join("cozy_anchor.db");
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS devices (
                node_id TEXT PRIMARY KEY,
                name TEXT,
                paired_at INTEGER
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                created_at INTEGER,
                updated_at INTEGER,
                device_id TEXT,
                deleted INTEGER DEFAULT 0,
                deleted_at INTEGER
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS todos (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT,
                due_date INTEGER,
                status TEXT DEFAULT 'pending',
                created_at INTEGER,
                updated_at INTEGER,
                device_id TEXT,
                deleted INTEGER DEFAULT 0,
                deleted_at INTEGER
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_log (
                device_id TEXT,
                data_type TEXT,
                last_sync_at INTEGER,
                PRIMARY KEY (device_id, data_type)
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS file_transfers (
                id TEXT PRIMARY KEY,
                filename TEXT,
                size_bytes INTEGER,
                blake3_hash TEXT,
                status TEXT DEFAULT 'pending',
                source_node TEXT,
                target_node TEXT,
                created_at INTEGER,
                completed_at INTEGER
            )",
            [],
        )?;
        Ok(())
    }

    pub fn add_device(&self, device: &Device) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO devices (node_id, name, paired_at) VALUES (?1, ?2, ?3)",
            params![device.node_id, device.name, device.paired_at],
        )?;
        Ok(())
    }

    pub fn list_devices(&self) -> Result<Vec<Device>> {
        let mut stmt = self.conn.prepare(
            "SELECT node_id, name, paired_at FROM devices ORDER BY paired_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Device {
                node_id: row.get(0)?,
                name: row.get(1)?,
                paired_at: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn remove_device(&self, node_id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM devices WHERE node_id = ?1", params![node_id])?;
        self.conn.execute("DELETE FROM sync_log WHERE device_id = ?1", params![node_id])?;
        Ok(())
    }

    pub fn create_message(&self, msg: &Message) -> Result<()> {
        self.conn.execute(
            "INSERT INTO messages (id, content, created_at, updated_at, device_id, deleted, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![msg.id, msg.content, msg.created_at, msg.updated_at, msg.device_id, msg.deleted as i64, msg.deleted_at],
        )?;
        Ok(())
    }

    pub fn list_messages(&self) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, created_at, updated_at, device_id, deleted, deleted_at
             FROM messages WHERE deleted = 0 ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Message {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                device_id: row.get(4)?,
                deleted: row.get::<_, i64>(5)? != 0,
                deleted_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_messages_since(&self, since: i64) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, created_at, updated_at, device_id, deleted, deleted_at
             FROM messages WHERE updated_at > ?1 ORDER BY updated_at"
        )?;
        let rows = stmt.query_map([since], |row| {
            Ok(Message {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                device_id: row.get(4)?,
                deleted: row.get::<_, i64>(5)? != 0,
                deleted_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn upsert_message(&self, msg: &Message) -> Result<()> {
        self.conn.execute(
            "INSERT INTO messages (id, content, created_at, updated_at, device_id, deleted, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                content=excluded.content,
                updated_at=excluded.updated_at,
                device_id=excluded.device_id,
                deleted=excluded.deleted,
                deleted_at=excluded.deleted_at
             WHERE excluded.updated_at > messages.updated_at",
            params![msg.id, msg.content, msg.created_at, msg.updated_at, msg.device_id, msg.deleted as i64, msg.deleted_at],
        )?;
        Ok(())
    }

    pub fn create_todo(&self, todo: &Todo) -> Result<()> {
        self.conn.execute(
            "INSERT INTO todos (id, title, content, due_date, status, created_at, updated_at, device_id, deleted, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![todo.id, todo.title, todo.content, todo.due_date, todo.status, todo.created_at, todo.updated_at, todo.device_id, todo.deleted as i64, todo.deleted_at],
        )?;
        Ok(())
    }

    pub fn list_todos(&self) -> Result<Vec<Todo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, due_date, status, created_at, updated_at, device_id, deleted, deleted_at
             FROM todos WHERE deleted = 0 ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Todo {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                due_date: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                device_id: row.get(7)?,
                deleted: row.get::<_, i64>(8)? != 0,
                deleted_at: row.get(9)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_todos_since(&self, since: i64) -> Result<Vec<Todo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, due_date, status, created_at, updated_at, device_id, deleted, deleted_at
             FROM todos WHERE updated_at > ?1 ORDER BY updated_at"
        )?;
        let rows = stmt.query_map([since], |row| {
            Ok(Todo {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                due_date: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                device_id: row.get(7)?,
                deleted: row.get::<_, i64>(8)? != 0,
                deleted_at: row.get(9)?,
            })
        })?;
        rows.collect()
    }

    pub fn upsert_todo(&self, todo: &Todo) -> Result<()> {
        self.conn.execute(
            "INSERT INTO todos (id, title, content, due_date, status, created_at, updated_at, device_id, deleted, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                title=excluded.title,
                content=excluded.content,
                due_date=excluded.due_date,
                status=excluded.status,
                updated_at=excluded.updated_at,
                device_id=excluded.device_id,
                deleted=excluded.deleted,
                deleted_at=excluded.deleted_at
             WHERE excluded.updated_at > todos.updated_at",
            params![todo.id, todo.title, todo.content, todo.due_date, todo.status, todo.created_at, todo.updated_at, todo.device_id, todo.deleted as i64, todo.deleted_at],
        )?;
        Ok(())
    }

    pub fn update_todo_status(&self, id: &str, status: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        self.conn.execute(
            "UPDATE todos SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, now, id],
        )?;
        Ok(())
    }

    pub fn delete_todo(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        self.conn.execute(
            "UPDATE todos SET deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn get_sync_log(&self, device_id: &str, data_type: &str) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT last_sync_at FROM sync_log WHERE device_id = ?1 AND data_type = ?2"
        )?;
        let result: Result<i64, _> = stmt.query_row(params![device_id, data_type], |row| row.get(0));
        Ok(result.unwrap_or(0))
    }

    pub fn set_sync_log(&self, device_id: &str, data_type: &str, ts: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sync_log (device_id, data_type, last_sync_at) VALUES (?1, ?2, ?3)",
            params![device_id, data_type, ts],
        )?;
        Ok(())
    }

    pub fn add_transfer(&self, transfer: &FileTransfer) -> Result<()> {
        self.conn.execute(
            "INSERT INTO file_transfers (id, filename, size_bytes, blake3_hash, status, source_node, target_node, created_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![transfer.id, transfer.filename, transfer.size_bytes, transfer.blake3_hash, transfer.status, transfer.source_node, transfer.target_node, transfer.created_at, transfer.completed_at],
        )?;
        Ok(())
    }

    pub fn list_transfers(&self) -> Result<Vec<FileTransfer>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filename, size_bytes, blake3_hash, status, source_node, target_node, created_at, completed_at
             FROM file_transfers ORDER BY created_at DESC LIMIT 50"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FileTransfer {
                id: row.get(0)?,
                filename: row.get(1)?,
                size_bytes: row.get(2)?,
                blake3_hash: row.get(3)?,
                status: row.get(4)?,
                source_node: row.get(5)?,
                target_node: row.get(6)?,
                created_at: row.get(7)?,
                completed_at: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn update_transfer_status(&self, id: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE file_transfers SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }
}
