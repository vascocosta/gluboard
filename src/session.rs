use anyhow::{Context, Result};
use bcrypt::DEFAULT_COST;
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use tokio::{
    fs::{File, read_to_string},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::RwLock,
};

const USERS_FILE: &str = "users.json";
const MESSAGES_FILE: &str = "messages.json";

pub struct Session {
    stream: BufReader<TcpStream>,
    app_state: Arc<AppState>,
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

    pub async fn login(&mut self) -> Result<LoginStatus> {
        let username = self.prompt("Username: ").await?;
        let password = self.prompt("Password: ").await?;

        let users = self.app_state.users.read().await;
        let user: &User = users
            .iter()
            .filter(|u| u.username == username)
            .collect::<Vec<&User>>()
            .first()
            .context("Could not find user")?;

        let valid_password = bcrypt::verify(password, &user.password)?;

        if !valid_password {
            Ok(LoginStatus::Failure)
        } else {
            Ok(LoginStatus::Success(user.username.clone()))
        }
    }

    pub async fn prompt(&mut self, text: &str) -> Result<String> {
        let mut answer = String::new();

        self.write(text).await?;
        self.stream.read_line(&mut answer).await?;

        Ok(answer.trim().to_owned())
    }

    async fn register(&mut self) -> Result<()> {
        let username = self.prompt("Choose a username: ").await?;
        let password = self.prompt("Choose a password: ").await?;

        let user = User {
            id: 1,
            username: username.to_owned(),
            password: bcrypt::hash(password, DEFAULT_COST)?,
        };

        self.app_state.users.write().await.push(user);

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        self.welcome().await.context("Could not perform welcome")?;

        let mut input = String::new();

        loop {
            self.stream
                .read_line(&mut input)
                .await
                .context("Could not read data from client")?;

            // TODO: Handle input by parsing it into a command.
            input.clear();
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

        loop {
            self.writeln("Commands:").await?;
            self.writeln("login | register | disconnect").await?;

            let option = self.prompt("> ").await?;

            self.writeln("").await?;

            match option.as_str() {
                "login" => {
                    loop {
                        match self.login().await.context("Could not validate login") {
                            Ok(LoginStatus::Success(username)) => {
                                println!("Successful login from user: {username}");
                                self.writeln("Login successful!").await?;
                                self.login_status = LoginStatus::Success(username);
                                break;
                            }
                            Ok(LoginStatus::Failure) => {
                                self.writeln("Login failed!").await?;

                                continue;
                            }
                            Err(e) => eprintln!("{e}"),
                        }
                    }

                    break Ok(());
                }
                "register" => match self.register().await.context("Could not register user") {
                    Ok(_) => {
                        self.app_state.save().await?;
                        println!("Successful user registration");
                    }
                    Err(e) => eprintln!("{e}"),
                },
                "disconnect" => self.stream.get_mut().shutdown().await?,
                _ => self.writeln("Invalid command!").await?,
            }
        }
    }

    async fn write(&mut self, data: &str) -> Result<()> {
        self.send(data, false).await
    }

    async fn writeln(&mut self, data: &str) -> Result<()> {
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

    async fn save(&self) -> Result<()> {
        let mut file = File::create(USERS_FILE).await?;
        let users = &*self.users.read().await; // * gets the inner value of the Lock.
        let users_json = serde_json::to_string_pretty(users)?;

        file.write_all(users_json.as_bytes()).await?;

        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
struct Message {
    id: i64,
    user: User,
    subject: String,
    body: String,
}

#[derive(Debug)]
pub enum LoginStatus {
    Success(String),
    Failure,
}
