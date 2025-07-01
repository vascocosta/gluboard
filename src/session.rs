use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use tokio::{
    fs::{File, read_to_string},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::RwLock,
};

use crate::commands::CommandHandler;

const USERS_FILE: &str = "users.json";
const MESSAGES_FILE: &str = "messages.json";

pub struct Session {
    stream: BufReader<TcpStream>,
    pub app_state: Arc<AppState>,
    pub login_status: LoginStatus,
}

impl Session {
    pub fn new(stream: TcpStream, app_state: Arc<AppState>) -> Self {
        Self {
            stream: BufReader::new(stream),
            app_state,
            login_status: LoginStatus::Failure,
        }
    }

    pub async fn prompt(&mut self, text: &str) -> Result<String> {
        let mut answer = String::new();

        self.write(text).await?;
        self.stream.read_line(&mut answer).await?;

        Ok(answer.trim().to_owned())
    }

    pub async fn run(&mut self) -> Result<()> {
        self.welcome().await.context("Could not perform welcome")?;

        let command_handler = CommandHandler::new();
        loop {
            let raw_command = self.prompt("> ").await?;
            command_handler.handle(&raw_command, self).await?;
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
        self.writeln("WELCOME TO THIS BBS").await?;
        self.writeln("").await?;

        self.writeln("Commands:").await?;
        self.writeln("login | register | disconnect").await?;
        self.writeln("").await?;

        let command_handler = CommandHandler::new();

        loop {
            let raw_command = self.prompt("> ").await?;

            if let Err(e) = command_handler.handle(&raw_command, self).await {
                self.writeln(&format!("{e}")).await?
            }
        }
    }

    pub async fn write(&mut self, data: &str) -> Result<()> {
        self.send(data, false).await
    }

    pub async fn writeln(&mut self, data: &str) -> Result<()> {
        self.send(data, true).await
    }
}

#[allow(dead_code)]
pub struct AppState {
    pub users: RwLock<Vec<User>>,
    pub messages: RwLock<Vec<Message>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(Vec::new()),
            messages: RwLock::new(Vec::new()),
        }
    }

    pub async fn from_file() -> Result<Self> {
        let users: Vec<User> = if Path::new(USERS_FILE).exists() {
            let users_json = read_to_string(USERS_FILE).await?;
            serde_json::from_str(&users_json)?
        } else {
            Vec::new()
        };

        let messages: Vec<Message> = if Path::new(MESSAGES_FILE).exists() {
            let messages_json = read_to_string(MESSAGES_FILE).await?;
            serde_json::from_str(&messages_json)?
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
