mod session;

use std::sync::Arc;

use anyhow::{Context, Result};
use session::{AppState, ConnectionManager};
use tokio::{net::TcpListener, spawn};

const ADDRESS: &str = "127.0.0.1:2323";

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind(ADDRESS).await?;
    let app_state = Arc::new(AppState::from_file().await.unwrap_or(AppState::new()));

    loop {
        match listener.accept().await.context("Client connection failed") {
            Ok((stream, address)) => {
                let app_state = Arc::clone(&app_state);

                println!("Connection from: {address}");

                spawn(async move {
                    let mut connection_manager = ConnectionManager::new(stream, app_state);

                    if let Err(e) = connection_manager.run().await {
                        eprintln!("{e}");
                    }
                });
            }
            Err(e) => eprintln!("{e}"),
        }
    }
}
