mod session;

use std::sync::Arc;

use anyhow::{Context, Result};
use session::{AppState, ConnectionManager};
use tokio::{net::TcpListener, spawn};

const ADDRESS: &str = "127.0.0.1:2323";

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind(ADDRESS).await?;

    loop {
        match listener.accept().await.context("Client connection failed") {
            Ok((stream, address)) => {
                spawn(async move {
                    let app_state = AppState::from_file().unwrap_or(AppState::new());
                    let connection_manager = ConnectionManager::new(stream, Arc::new(app_state));

                    connection_manager.run().await;
                });
            }
            Err(e) => eprintln!("{e}"),
        }
    }

    Ok(())
}
