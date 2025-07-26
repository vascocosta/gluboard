use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, read, read_to_string},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::{Mutex, RwLock},
};

use crate::{ansi::AnsiStyle, commands::CommandHandler};

const BANNER_FILE: &str = "banner.ans";
const USERS_FILE: &str = "users.json";
const MESSAGES_FILE: &str = "messages.json";

pub struct Session {
    stream: BufReader<TcpStream>,
    pub app_state: Arc<AppState>,
    pub login_status: LoginStatus,
    command_handler: Arc<Mutex<CommandHandler>>,
}

impl Session {
    pub fn new(
        stream: TcpStream,
        app_state: Arc<AppState>,
        command_handler: Arc<Mutex<CommandHandler>>,
    ) -> Self {
        Self {
            stream: BufReader::new(stream),
            app_state,
            login_status: LoginStatus::Failure,
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
        self.welcome().await.context("Could not perform welcome")?;

        let command_handler = Arc::clone(&self.command_handler);

        loop {
            let raw_command = self.prompt("> ", None).await?;
            command_handler
                .lock()
                .await
                .handle(&raw_command, self)
                .await?;
        }
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

    pub async fn welcome(&mut self) -> Result<()> {
        if let Ok(banner) = read(BANNER_FILE).await {
            self.writeln(&String::from_utf8_lossy(&banner), None)
                .await?;
            self.writeln("", None).await?;
        }

        self.writeln("WELCOME TO THIS BBS", None).await?;
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

            if let Err(e) = command_handler
                .lock()
                .await
                .handle(&raw_command, self)
                .await
            {
                self.writeln(&format!("{e}"), None).await?
            }
        }
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

#[allow(dead_code)]
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
pub enum LoginStatus {
    Success(String),
    Failure,
}

pub enum AppStateKind {
    Users,
    Messages,
}
