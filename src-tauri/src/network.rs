use anyhow::Result;
use iroh::{Endpoint, NodeAddr, NodeId};
use std::sync::Arc;
use tokio::sync::mpsc;

pub const ALPN: &[u8] = b"cozy-anchor/1";

pub struct P2PNode {
    endpoint: Arc<Endpoint>,
}

impl P2PNode {
    pub async fn new() -> Result<(Self, mpsc::UnboundedReceiver<iroh::endpoint::Connection>)> {
        let endpoint = Arc::new(Endpoint::builder().bind().await?);
        let (tx, rx) = mpsc::unbounded_channel();

        let ep = endpoint.clone();
        tokio::spawn(async move {
            while let Some(incoming) = ep.accept().await {
                let tx = tx.clone();
                tokio::spawn(async move {
                    match incoming.accept() {
                        Ok(connecting) => match connecting.await {
                            Ok(conn) => {
                                let _ = tx.send(conn);
                            }
                            Err(e) => eprintln!("Connection failed: {}", e),
                        },
                        Err(e) => eprintln!("Accept failed: {}", e),
                    }
                });
            }
        });

        Ok((Self { endpoint }, rx))
    }

    pub fn node_id(&self) -> String {
        self.endpoint.node_id().to_string()
    }

    pub async fn connect(&self, target_node_id: &str) -> Result<iroh::endpoint::Connection> {
        let node_id: NodeId = target_node_id.parse()?;
        let addr = NodeAddr::from(node_id);
        let conn = self.endpoint.connect(addr, ALPN).await?;
        Ok(conn)
    }
}
