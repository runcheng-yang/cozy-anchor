#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod models;
mod network;
mod protocol;
mod sync;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::db::Db;
use crate::models::FileTransfer;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Db>>,
    pub p2p: Arc<Mutex<network::P2PNode>>,
    pub receive_dir: Arc<Mutex<Option<PathBuf>>>,
}

#[cfg(target_os = "windows")]
fn com_sta_init() {
    unsafe {
        #[link(name = "ole32")]
        extern "system" {
            fn CoInitializeEx(pvReserved: *mut std::ffi::c_void, dwCoInit: u32) -> i32;
        }
        const COINIT_APARTMENTTHREADED: u32 = 0x2;
        CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    com_sta_init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    rt.block_on(async {
        let (p2p, mut incoming_rx) = network::P2PNode::new().await.expect("Failed to start P2P node");

        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .expect("Cannot find home directory");
        let app_dir = PathBuf::from(home).join(".cozy-anchor");
        std::fs::create_dir_all(&app_dir).expect("Failed to create app dir");
        let db = db::Db::new(app_dir).expect("Failed to open DB");

        let state = AppState {
            db: Arc::new(Mutex::new(db)),
            p2p: Arc::new(Mutex::new(p2p)),
            receive_dir: Arc::new(Mutex::new(None)),
        };

        let state_clone = state.clone();
        tokio::spawn(async move {
            while let Some(conn) = incoming_rx.recv().await {
                let s = state_clone.clone();
                tokio::spawn(async move {
                    handle_connection(s, conn).await;
                });
            }
        });

        tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_dialog::init())
            .manage(state)
            .invoke_handler(tauri::generate_handler![
                commands::get_node_id,
                commands::list_devices,
                commands::pair_device,
                commands::remove_device,
                commands::list_messages,
                commands::send_message,
                commands::create_todo,
                commands::list_todos,
                commands::update_todo_status,
                commands::delete_todo,
                commands::sync_with_device,
                commands::send_file,
                commands::list_transfers,
                commands::set_receive_dir,
                commands::get_receive_dir,
            ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    });
}

async fn handle_connection(state: AppState, conn: iroh::endpoint::Connection) {
    while let Ok((mut send, mut recv)) = conn.accept_bi().await {
        let mut len_buf = [0u8; 4];
        if recv.read_exact(&mut len_buf).await.is_err() {
            continue;
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut data = vec![0u8; len];
        if recv.read_exact(&mut data).await.is_err() {
            continue;
        }

        if let Ok(msg) = serde_json::from_slice::<crate::protocol::SyncMessage>(&data) {
            match msg {
                crate::protocol::SyncMessage::Pull { tables } => {
                    let (messages, todos) = {
                        let db = state.db.lock().await;
                        let msgs = db.get_messages_since(tables.messages).unwrap_or_default();
                        let tds = db.get_todos_since(tables.todos).unwrap_or_default();
                        (msgs, tds)
                    };

                    let data = crate::protocol::SyncMessage::Data {
                        tables: crate::protocol::DataTables { messages, todos },
                    };
                    if crate::sync::send_json(&mut send, &data).await.is_err() { continue; }

                    if let Ok(response) = crate::sync::recv_json(&mut recv).await {
                        if let crate::protocol::SyncMessage::Data { tables } = response {
                            let db = state.db.lock().await;
                            for msg in tables.messages {
                                let _ = db.upsert_message(&msg);
                            }
                            for todo in tables.todos {
                                let _ = db.upsert_todo(&todo);
                            }
                        }
                        let done = crate::protocol::SyncMessage::Done;
                        let _ = crate::sync::send_json(&mut send, &done).await;
                    }
                }
                crate::protocol::SyncMessage::Data { tables } => {
                    let db = state.db.lock().await;
                    for msg in tables.messages {
                        let _ = db.upsert_message(&msg);
                    }
                    for todo in tables.todos {
                        let _ = db.upsert_todo(&todo);
                    }
                    drop(db);
                    let done = crate::protocol::SyncMessage::Done;
                    let _ = crate::sync::send_json(&mut send, &done).await;
                }
                _ => {}
            }
            continue;
        }

        if let Ok(msg) = serde_json::from_slice::<crate::protocol::FileProtocol>(&data) {
            if let Err(e) = handle_file_message(state.clone(), &mut send, &mut recv, msg).await {
                eprintln!("File handler error: {}", e);
            }
        }
    }
}

async fn handle_file_message(
    state: AppState,
    send: &mut (impl AsyncWriteExt + Unpin),
    recv: &mut (impl AsyncReadExt + Unpin),
    first_msg: crate::protocol::FileProtocol,
) -> anyhow::Result<()> {
    match first_msg {
        crate::protocol::FileProtocol::Offer { transfer_id, filename, size } => {
            let receive_dir = {
                let dir = state.receive_dir.lock().await;
                dir.clone()
            };

            if let Some(dir) = receive_dir {
                let accept = crate::protocol::FileProtocol::Accept {
                    transfer_id: transfer_id.clone(),
                };
                let data = serde_json::to_vec(&accept)?;
                let len = data.len() as u32;
                send.write_all(&len.to_be_bytes()).await?;
                send.write_all(&data).await?;

                let mut size_buf = [0u8; 8];
                recv.read_exact(&mut size_buf).await?;
                let file_size = u64::from_be_bytes(size_buf) as usize;
                let mut file_data = vec![0u8; file_size];
                recv.read_exact(&mut file_data).await?;

                let save_path = dir.join(&filename);
                std::fs::write(&save_path, &file_data)?;

                let my_node_id = {
                    let p2p = state.p2p.lock().await;
                    p2p.node_id()
                };

                let transfer = FileTransfer {
                    id: transfer_id.clone(),
                    filename,
                    size_bytes: size,
                    blake3_hash: None,
                    status: "received".to_string(),
                    source_node: "unknown".to_string(),
                    target_node: my_node_id,
                    created_at: Utc::now().timestamp_millis(),
                    completed_at: Some(Utc::now().timestamp_millis()),
                };

                let db = state.db.lock().await;
                db.add_transfer(&transfer)?;
            } else {
                let decline = crate::protocol::FileProtocol::Decline {
                    transfer_id: transfer_id.clone(),
                };
                let data = serde_json::to_vec(&decline)?;
                let len = data.len() as u32;
                send.write_all(&len.to_be_bytes()).await?;
                send.write_all(&data).await?;
            }
        }
        _ => {}
    }
    Ok(())
}
