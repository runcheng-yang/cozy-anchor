use tauri::State;
use tauri_plugin_dialog::DialogExt;
use crate::{AppState, models::*};
use chrono::Utc;

#[tauri::command]
pub async fn get_node_id(state: State<'_, AppState>) -> Result<String, String> {
    let p2p = state.p2p.lock().await;
    Ok(p2p.node_id())
}

#[tauri::command]
pub async fn list_devices(state: State<'_, AppState>) -> Result<Vec<Device>, String> {
    let db = state.db.lock().await;
    db.list_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pair_device(state: State<'_, AppState>, node_id: String, name: String) -> Result<(), String> {
    let device = Device {
        node_id,
        name,
        paired_at: Utc::now().timestamp_millis(),
    };
    let db = state.db.lock().await;
    db.add_device(&device).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_device(state: State<'_, AppState>, node_id: String) -> Result<(), String> {
    let db = state.db.lock().await;
    db.remove_device(&node_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_messages(state: State<'_, AppState>) -> Result<Vec<Message>, String> {
    let db = state.db.lock().await;
    db.list_messages().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_message(state: State<'_, AppState>, content: String, target_node: String) -> Result<(), String> {
    let msg = Message {
        id: uuid::Uuid::new_v4().to_string(),
        content: content.clone(),
        created_at: Utc::now().timestamp_millis(),
        updated_at: Utc::now().timestamp_millis(),
        device_id: None,
        deleted: false,
        deleted_at: None,
    };

    {
        let db = state.db.lock().await;
        db.create_message(&msg).map_err(|e| e.to_string())?;
    }

    let p2p = state.p2p.lock().await;
    match p2p.connect(&target_node).await {
        Ok(conn) => {
            drop(p2p);
            if let Ok((mut send, mut recv)) = conn.open_bi().await {
                let push = crate::protocol::SyncMessage::Data {
                    tables: crate::protocol::DataTables {
                        messages: vec![msg],
                        todos: vec![],
                    },
                };
                let _ = crate::sync::send_json(&mut send, &push).await;
                let _ = crate::sync::recv_json(&mut recv).await;
            }
        }
        Err(e) => eprintln!("Push failed: {}", e),
    }

    Ok(())
}

#[tauri::command]
pub async fn create_todo(state: State<'_, AppState>, title: String) -> Result<(), String> {
    let todo = Todo {
        id: uuid::Uuid::new_v4().to_string(),
        title,
        content: None,
        due_date: None,
        status: "pending".to_string(),
        created_at: Utc::now().timestamp_millis(),
        updated_at: Utc::now().timestamp_millis(),
        device_id: None,
        deleted: false,
        deleted_at: None,
    };
    let db = state.db.lock().await;
    db.create_todo(&todo).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_todos(state: State<'_, AppState>) -> Result<Vec<Todo>, String> {
    let db = state.db.lock().await;
    db.list_todos().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_todo_status(state: State<'_, AppState>, id: String, status: String) -> Result<(), String> {
    let db = state.db.lock().await;
    db.update_todo_status(&id, &status).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_todo(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().await;
    db.delete_todo(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_with_device(state: State<'_, AppState>, target_node: String) -> Result<(), String> {
    let (last_msg, last_todo, our_messages, our_todos) = {
        let db = state.db.lock().await;
        let last_msg = db.get_sync_log(&target_node, "messages").unwrap_or(0);
        let last_todo = db.get_sync_log(&target_node, "todos").unwrap_or(0);
        let msgs = db.get_messages_since(last_msg).unwrap_or_default();
        let todos = db.get_todos_since(last_todo).unwrap_or_default();
        (last_msg, last_todo, msgs, todos)
    };

    let p2p = state.p2p.lock().await;
    let (their_messages, their_todos) = crate::sync::sync_exchange(
        &*p2p, &target_node, last_msg, last_todo, our_messages, our_todos
    ).await.map_err(|e| e.to_string())?;
    drop(p2p);

    {
        let db = state.db.lock().await;
        for msg in their_messages {
            db.upsert_message(&msg).map_err(|e| e.to_string())?;
        }
        for todo in their_todos {
            db.upsert_todo(&todo).map_err(|e| e.to_string())?;
        }
        let now = chrono::Utc::now().timestamp_millis();
        db.set_sync_log(&target_node, "messages", now).map_err(|e| e.to_string())?;
        db.set_sync_log(&target_node, "todos", now).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn send_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    target_node: String,
) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_file(move |path| {
        let _ = tx.send(path);
    });
    let file_path = rx.await.map_err(|_| "Dialog cancelled")?
        .ok_or("No file selected")?;

    let path = file_path.as_path().ok_or("Invalid file path")?;
    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let size = metadata.len() as i64;

    let my_node_id = {
        let p2p = state.p2p.lock().await;
        p2p.node_id()
    };

    let transfer = FileTransfer {
        id: uuid::Uuid::new_v4().to_string(),
        filename: filename.clone(),
        size_bytes: size,
        blake3_hash: None,
        status: "sending".to_string(),
        source_node: my_node_id,
        target_node: target_node.clone(),
        created_at: Utc::now().timestamp_millis(),
        completed_at: None,
    };

    {
        let db = state.db.lock().await;
        db.add_transfer(&transfer).map_err(|e| e.to_string())?;
    }

    let content = std::fs::read(&path).map_err(|e| e.to_string())?;

    let p2p = state.p2p.lock().await;
    let conn = p2p.connect(&target_node).await.map_err(|e| e.to_string())?;
    drop(p2p);

    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?;

    let offer = crate::protocol::FileProtocol::Offer {
        transfer_id: transfer.id.clone(),
        filename,
        size,
    };
    let data = serde_json::to_vec(&offer).map_err(|e| e.to_string())?;
    let len = data.len() as u32;
    send.write_all(&len.to_be_bytes()).await.map_err(|e| e.to_string())?;
    send.write_all(&data).await.map_err(|e| e.to_string())?;

    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf).await.map_err(|e| e.to_string())?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut resp = vec![0u8; len];
    recv.read_exact(&mut resp).await.map_err(|e| e.to_string())?;
    let resp: crate::protocol::FileProtocol = serde_json::from_slice(&resp).map_err(|e| e.to_string())?;

    match resp {
        crate::protocol::FileProtocol::Accept { .. } => {
            send.write_all(&(content.len() as u64).to_be_bytes()).await.map_err(|e| e.to_string())?;
            send.write_all(&content).await.map_err(|e| e.to_string())?;

            let db = state.db.lock().await;
            db.update_transfer_status(&transfer.id, "sent").map_err(|e| e.to_string())?;
        }
        _ => {
            let db = state.db.lock().await;
            db.update_transfer_status(&transfer.id, "declined").map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn list_transfers(state: State<'_, AppState>) -> Result<Vec<FileTransfer>, String> {
    let db = state.db.lock().await;
    db.list_transfers().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_receive_dir(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().set_title("选择接收文件夹").pick_folder(move |path| {
        let _ = tx.send(path);
    });
    let folder = rx.await.map_err(|_| "Dialog cancelled")?
        .ok_or("No folder selected")?;

    let path = folder.as_path().ok_or("Invalid folder path")?.to_path_buf();
    let mut dir = state.receive_dir.lock().await;
    *dir = Some(path);
    Ok(())
}

#[tauri::command]
pub async fn get_receive_dir(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let dir = state.receive_dir.lock().await;
    Ok(dir.as_ref().map(|p| p.to_string_lossy().to_string()))
}
