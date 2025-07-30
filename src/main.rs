mod ansi;
mod commands;
mod config;
mod session;

use std::sync::Arc;

use anyhow::{Context, Result};
use session::{AppState, Session};
use tokio::{net::TcpListener, spawn, sync::Mutex};

use crate::{
    commands::{CommandHandler, HelpCmd, LoginCmd, MessageCmd, QuitCmd, RegisterCmd},
    config::Config,
};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(Config::from_file().await?);
    let hostname = &config.hostname;
    let port = config.port;

    match AppState::from_file().await {
        Ok(app_state) => {
            let config = Arc::clone(&config);
            let app_state = Arc::new(app_state);
            let listener = TcpListener::bind(format!("{hostname}:{port}")).await?;
            let command_handler = Arc::new(Mutex::new(CommandHandler::new()));

            {
                let mut lock = command_handler.lock().await;

                lock.add_welcome_cmd(LoginCmd);
                lock.add_welcome_cmd(RegisterCmd);
                lock.add_welcome_cmd(QuitCmd);
                lock.add_message_cmd(MessageCmd);
                lock.add_message_cmd(QuitCmd);

                let command_handler_clone = lock.clone();
                lock.add_welcome_cmd(HelpCmd {
                    command_handler: command_handler_clone,
                });
                let command_handler_clone = lock.clone();
                lock.add_message_cmd(HelpCmd {
                    command_handler: command_handler_clone,
                });
            }

            loop {
                match listener.accept().await.context("Client connection failed") {
                    Ok((stream, address)) => {
                        let config = Arc::clone(&config);
                        let app_state = Arc::clone(&app_state);
                        let command_handler = Arc::clone(&command_handler);

                        println!("Connection from: {address}");

                        spawn(async move {
                            let mut session =
                                Session::new(stream, config, app_state, command_handler);

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
