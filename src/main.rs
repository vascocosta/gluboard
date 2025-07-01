mod commands;
mod session;

use std::sync::Arc;

use anyhow::{Context, Result};
use session::{AppState, Session};
use tokio::{net::TcpListener, spawn};

const ADDRESS: &str = "127.0.0.1:2323";

#[tokio::main]
async fn main() -> Result<()> {
    match AppState::from_file().await {
        Ok(app_state) => {
            let app_state = Arc::new(app_state);
            let listener = TcpListener::bind(ADDRESS).await?;

            loop {
                match listener.accept().await.context("Client connection failed") {
                    Ok((stream, address)) => {
                        let app_state = Arc::clone(&app_state);

                        println!("Connection from: {address}");

                        spawn(async move {
                            let mut session = Session::new(stream, app_state);

                            if let Err(e) = session.run().await {
                                eprintln!("{e}");
                            }
                        });
                    }
                    Err(e) => eprintln!("{e}"),
                }
            }
        }
        Err(e) => {
            eprintln!("{e}");
        }
    }

    Ok(())
}
