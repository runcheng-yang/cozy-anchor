use crate::models::{Message, Todo};
use crate::protocol::{DataTables, SyncMessage, SyncTables};
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn send_json<W: AsyncWriteExt + Unpin>(writer: &mut W, msg: &SyncMessage) -> Result<()> {
    let data = serde_json::to_vec(msg)?;
    let len = data.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&data).await?;
    Ok(())
}

pub async fn recv_json<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<SyncMessage> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut data = vec![0u8; len];
    reader.read_exact(&mut data).await?;
    let msg = serde_json::from_slice(&data)?;
    Ok(msg)
}

/// 纯网络交换：发送本地增量，接收对方增量。不操作数据库。
pub async fn sync_exchange(
    node: &crate::network::P2PNode,
    target_node: &str,
    last_msg: i64,
    last_todo: i64,
    our_messages: Vec<Message>,
    our_todos: Vec<Todo>,
) -> Result<(Vec<Message>, Vec<Todo>)> {
    let conn = node.connect(target_node).await?;
    let (mut send, mut recv) = conn.open_bi().await?;

    let pull = SyncMessage::Pull {
        tables: SyncTables {
            messages: last_msg,
            todos: last_todo,
        },
    };
    send_json(&mut send, &pull).await?;

    let response = recv_json(&mut recv).await?;
    let (their_messages, their_todos) = match response {
        SyncMessage::Data { tables } => (tables.messages, tables.todos),
        _ => (vec![], vec![]),
    };

    let data = SyncMessage::Data {
        tables: DataTables {
            messages: our_messages,
            todos: our_todos,
        },
    };
    send_json(&mut send, &data).await?;

    let _ = recv_json(&mut recv).await?;

    Ok((their_messages, their_todos))
}
