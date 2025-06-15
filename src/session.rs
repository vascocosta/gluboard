use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs::read_to_string, path::Path, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::RwLock,
};

const USERS_FILE: &str = "users.json";
const MESSAGES_FILE: &str = "messages.json";

pub struct ConnectionManager {
    stream: BufReader<TcpStream>,
    app_state: Arc<AppState>,
}

impl ConnectionManager {
    pub fn new(stream: TcpStream, app_state: Arc<AppState>) -> Self {
        Self {
            stream: BufReader::new(stream),
            app_state,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        self.welcome()
            .await
            .context("Could not send welcome message")?;

        let mut input = String::new();

        loop {
            self.stream.read_line(&mut input).await?;
            // TODO: Handle input by parsing it into a command.
            input.clear();
        }
    }

    async fn send(&mut self, data: &str) -> Result<()> {
        self.stream
            .get_mut()
            .write_all(format!("{data}\r\n").as_bytes())
            .await
            .context("Could not send data to client")?;

        self.stream
            .flush()
            .await
            .context("Could not send data to client")
    }

    pub async fn welcome(&mut self) -> Result<()> {
        self.send("WELCOME TO GLUON'S BBS").await
    }
}

pub struct AppState {
    users: RwLock<Vec<User>>,
    messages: RwLock<Vec<Message>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(Vec::new()),
            messages: RwLock::new(Vec::new()),
        }
    }

    pub fn from_file() -> Result<Self> {
        let users: Vec<User> = if Path::new(USERS_FILE).exists() {
            let users_json = read_to_string(USERS_FILE)?;
            serde_json::from_str(&users_json)?
        } else {
            Vec::new()
        };

        let messages: Vec<Message> = if Path::new(MESSAGES_FILE).exists() {
            let messages_json = read_to_string(MESSAGES_FILE)?;
            serde_json::from_str(&messages_json)?
        } else {
            Vec::new()
        };

        Ok(Self {
            users: RwLock::new(users),
            messages: RwLock::new(messages),
        })
    }
}

#[derive(Deserialize, Serialize)]
struct User {
    id: i64,
    username: String,
    password: String,
}

#[derive(Deserialize, Serialize)]
struct Message {
    id: i64,
    user: User,
    subject: String,
    body: String,
}
