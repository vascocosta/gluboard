use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, read, read_to_string},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::{Mutex, RwLock},
};

use crate::{ansi::AnsiStyle, commands::CommandHandler, config::Config};

const USERS_FILE: &str = "users.json";
const MESSAGES_FILE: &str = "messages.json";

pub struct Session {
    pub stream: BufReader<TcpStream>,
    config: Arc<Config>,
    pub app_state: Arc<AppState>,
    pub status: SessionStatus,
    command_handler: Arc<Mutex<CommandHandler>>,
}

impl Session {
    pub fn new(
        stream: TcpStream,
        config: Arc<Config>,
        app_state: Arc<AppState>,
        command_handler: Arc<Mutex<CommandHandler>>,
    ) -> Self {
        Self {
            stream: BufReader::new(stream),
            config,
            app_state,
            status: SessionStatus::LoggedOff,
            command_handler,
        }
    }

    pub async fn prompt(&mut self, text: &str, style: Option<AnsiStyle>) -> Result<String> {
        let mut answer = String::new();

        self.write(text, style).await?;
        self.stream.read_line(&mut answer).await?;

        Ok(answer.trim().to_owned())
    }

    pub async fn run(&mut self) -> Result<()> {
        if let Some(banner_file) = &self.config.banner_file {
            if let Ok(banner_data) = read(banner_file).await {
                self.writeln(&String::from_utf8_lossy(&banner_data), None)
                    .await?;
                self.writeln("", None).await?;
            }
        }

        if let Some(welcome_msg) = self.config.welcome_msg.clone() {
            self.writeln(&welcome_msg, None).await?;
            self.writeln("", None).await?;
        }

        self.writeln("", None).await?;

        let commands: Vec<String> = self
            .command_handler
            .lock()
            .await
            .welcome_commands
            .keys()
            .map(|k| k.to_lowercase())
            .collect();

        self.writeln("Commands:", None).await?;
        self.writeln(&commands.join(" | "), None).await?;
        self.writeln("", None).await?;

        let command_handler = Arc::clone(&self.command_handler);

        loop {
            let raw_command = self.prompt("> ", None).await?;

            match command_handler
                .lock()
                .await
                .handle(&raw_command, self)
                .await
            {
                Ok(_) => {
                    if let SessionStatus::Disconnected = self.status {
                        break;
                    }
                }
                Err(e) => self.writeln(&format!("{e}"), None).await?,
            }
        }

        Ok(())
    }

    async fn send(&mut self, data: &str, newline: bool) -> Result<()> {
        self.stream
            .get_mut()
            .write_all(format!("{data}{}", if newline { "\r\n" } else { "" }).as_bytes())
            .await
            .context("Could not send data to client")?;

        self.stream
            .flush()
            .await
            .context("Could not send data to client")
    }

    pub async fn write(&mut self, data: &str, style: Option<AnsiStyle>) -> Result<()> {
        match style {
            None => self.send(data, false).await,
            Some(style) => self.send(&style.apply(data), false).await,
        }
    }

    pub async fn writeln(&mut self, data: &str, style: Option<AnsiStyle>) -> Result<()> {
        match style {
            None => self.send(data, true).await,
            Some(style) => self.send(&style.apply(data), true).await,
        }
    }
}

pub struct AppState {
    pub users: RwLock<Vec<User>>,
    pub messages: RwLock<Vec<Message>>,
}

impl AppState {
    pub async fn from_file() -> Result<Self> {
        let users: Vec<User> = if Path::new(USERS_FILE).exists() {
            let users_json = read_to_string(USERS_FILE).await?;
            serde_json::from_str(&users_json).context("Could not read users")?
        } else {
            Vec::new()
        };

        let messages: Vec<Message> = if Path::new(MESSAGES_FILE).exists() {
            let messages_json = read_to_string(MESSAGES_FILE).await?;
            serde_json::from_str(&messages_json).context("Could not read messages")?
        } else {
            Vec::new()
        };

        Ok(Self {
            users: RwLock::new(users),
            messages: RwLock::new(messages),
        })
    }

    pub async fn save(&self, kind: AppStateKind) -> Result<()> {
        match kind {
            AppStateKind::Users => {
                let mut file = File::create(USERS_FILE).await?;
                let users = &*self.users.read().await; // * gets the inner value of the Lock.
                let users_json = serde_json::to_string_pretty(users)?;

                file.write_all(users_json.as_bytes()).await?;
            }
            AppStateKind::Messages => {
                let mut file = File::create(MESSAGES_FILE).await?;
                let messages = &*self.messages.read().await; // * gets the inner value of the Lock.
                let messages_json = serde_json::to_string_pretty(messages)?;

                file.write_all(messages_json.as_bytes()).await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Message {
    pub id: i64,
    pub username: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug)]
pub enum SessionStatus {
    LoggedOn(String),
    LoggedOff,
    Disconnected,
}

pub enum AppStateKind {
    Users,
    Messages,
}
