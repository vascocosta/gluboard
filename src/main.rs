mod commands;
mod session;

use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use session::{AppState, Session};
use tokio::{net::TcpListener, spawn};

use crate::commands::{Command, CommandHandler, Login, Messages, Register};

const ADDRESS: &str = "127.0.0.1:2323";

#[tokio::main]
async fn main() -> Result<()> {
    match AppState::from_file().await {
        Ok(app_state) => {
            let app_state = Arc::new(app_state);
            let listener = TcpListener::bind(ADDRESS).await?;

            let mut welcome_commands: HashMap<&'static str, Arc<dyn Command + Send + Sync>> =
                HashMap::new();

            commands::insert_command(Login, &mut welcome_commands);
            commands::insert_command(Register, &mut welcome_commands);

            let mut message_commands: HashMap<&'static str, Arc<dyn Command + Send + Sync>> =
                HashMap::new();

            commands::insert_command(Messages, &mut message_commands);

            let command_handler = Arc::new(CommandHandler::new(welcome_commands, message_commands));

            loop {
                match listener.accept().await.context("Client connection failed") {
                    Ok((stream, address)) => {
                        let app_state = Arc::clone(&app_state);
                        let command_handler = Arc::clone(&command_handler);

                        println!("Connection from: {address}");

                        spawn(async move {
                            let mut session = Session::new(stream, app_state, command_handler);

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
