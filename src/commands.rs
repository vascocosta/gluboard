use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;
use bcrypt::DEFAULT_COST;

use crate::session::{AppStateKind, LoginStatus, Message, Session, User};

pub struct CommandHandler {
    welcome_commands: HashMap<&'static str, Arc<dyn Command + Send + Sync>>,
    message_commands: HashMap<&'static str, Arc<dyn Command + Send + Sync>>,
}

impl CommandHandler {
    pub fn new(
        welcome_commands: HashMap<&'static str, Arc<dyn Command + Send + Sync>>,
        message_commands: HashMap<&'static str, Arc<dyn Command + Send + Sync>>,
    ) -> Self {
        Self {
            welcome_commands,
            message_commands,
        }
    }

    pub async fn handle(&self, raw_command: &str, session: &mut Session) -> Result<()> {
        let mut parts = raw_command.split_whitespace();
        let name = parts.next().context("Invalid command")?;
        let args: Vec<&str> = parts.collect();

        match session.login_status {
            LoginStatus::Failure => {
                self.welcome_commands
                    .get(name)
                    .context("Unknown command")?
                    .execute(session, if args.is_empty() { None } else { Some(&args) })
                    .await
            }
            LoginStatus::Success(_) => {
                self.message_commands
                    .get(name)
                    .context("Unknown command")?
                    .execute(session, if args.is_empty() { None } else { Some(&args) })
                    .await
            }
        }
    }
}

#[allow(dead_code)]
#[async_trait]
pub trait Command {
    fn names() -> &'static [&'static str]
    where
        Self: Sized;
    async fn execute(&self, session: &mut Session, args: Option<&[&str]>) -> Result<()>;
    fn help(&self) -> String;
}

pub struct Login;

#[async_trait]
impl Command for Login {
    fn names() -> &'static [&'static str] {
        &["login"]
    }

    async fn execute(&self, session: &mut Session, _: Option<&[&str]>) -> Result<()> {
        loop {
            let username = session.prompt("Username: ").await?;
            let password = session.prompt("Password: ").await?;

            let valid_password = {
                let users = session.app_state.users.read().await;
                let user: &User = users
                    .iter()
                    .filter(|u| u.username == username)
                    .collect::<Vec<&User>>()
                    .first()
                    .context("Could not find user")?;

                bcrypt::verify(password, &user.password).context("Invalid password")?
            };

            if !valid_password {
                session.login_status = LoginStatus::Failure;
                session.writeln("Login failed").await?;
            } else {
                session.login_status = LoginStatus::Success(username);
                session.writeln("Login successful").await?;
                break;
            }
        }

        Ok(())
    }

    fn help(&self) -> String {
        todo!()
    }
}

#[derive(Clone)]
pub struct Register;

impl Register {
    async fn generate_id(&self, session: &mut Session) -> Option<i64> {
        let users = &*session.app_state.users.read().await;

        Some(users.last()?.id + 1)
    }
}

#[async_trait]
impl Command for Register {
    fn names() -> &'static [&'static str] {
        &["register"]
    }

    async fn execute(&self, session: &mut Session, _: Option<&[&str]>) -> Result<()> {
        let username = session.prompt("Choose a username: ").await?;
        let password = session.prompt("Choose a password: ").await?;

        let user = User {
            id: self.generate_id(session).await.unwrap_or_default(),
            username: username.to_owned(),
            password: bcrypt::hash(password, DEFAULT_COST).context("Could not register user")?,
        };

        session.app_state.users.write().await.push(user);
        session.app_state.save(AppStateKind::Users).await?;
        session.login_status = LoginStatus::Success(username);
        session.writeln("Registration successful").await?;
        session.writeln("Login successful").await?;

        Ok(())
    }

    fn help(&self) -> String {
        todo!()
    }
}

#[derive(Clone)]
pub struct Messages;

impl Messages {
    async fn generate_id(&self, session: &mut Session) -> Option<i64> {
        let messages = &*session.app_state.messages.read().await;

        Some(messages.last()?.id + 1)
    }
}

#[async_trait]
impl Command for Messages {
    fn names() -> &'static [&'static str] {
        &["message", "messages", "msg"]
    }

    async fn execute(&self, session: &mut Session, args: Option<&[&str]>) -> Result<()> {
        match args {
            None => session.writeln("No sub commands").await,
            Some([sub_command]) => match *sub_command {
                "list" => {
                    let messages = {
                        let guard = session.app_state.messages.read().await;
                        guard.clone()
                    };

                    for message in messages {
                        session
                            .writeln(&format!(
                                "{} {} {}",
                                message.id, message.username, message.subject
                            ))
                            .await?;
                    }

                    Ok(())
                }
                "new" => {
                    let subject = session.prompt("Subject: ").await?;
                    let mut body = String::new();

                    session
                        .write("\r\nWrite your message. Type \".\" on a line by its own to finish.\r\n\r\n")
                        .await?;

                    while let Ok(line) = session.prompt("").await {
                        if line.trim() != "." {
                            body = format!("{}{}\r\n", body, line);
                        } else {
                            break;
                        }
                    }

                    let username = match &session.login_status {
                        LoginStatus::Success(username) => username.to_owned(),
                        LoginStatus::Failure => todo!(),
                    };

                    let message = Message {
                        id: self.generate_id(session).await.unwrap_or_default(),
                        username,
                        subject,
                        body,
                    };

                    session.app_state.messages.write().await.push(message);
                    session.app_state.save(AppStateKind::Messages).await?;

                    Ok(())
                }
                _ => session.writeln("Unknown sub command").await,
            },
            Some([sub_command, sub_arg]) => match *sub_command {
                "read" => {
                    let message = {
                        let messages = &*session.app_state.messages.read().await;
                        let index: i64 = sub_arg.parse()?;

                        messages
                            .get(index as usize)
                            .context("Invalid message id")?
                            .to_owned()
                    };

                    session
                        .writeln(&format!(
                            "Subject: {}\r\n\r\n{}",
                            message.subject, message.body
                        ))
                        .await
                }
                _ => session.writeln("Unknown sub command").await,
            },
            Some(&[]) | Some(&[_, _, _, ..]) => session.writeln("Show usage").await,
        }
    }

    fn help(&self) -> String {
        todo!()
    }
}

pub fn insert_command<C>(
    command: C,
    map: &mut HashMap<&'static str, Arc<dyn Command + Send + Sync>>,
) where
    C: Command + Send + Sync + 'static,
{
    let command = Arc::new(command);

    for alias in C::names() {
        let command_clone = Arc::clone(&command);

        map.insert(&alias, command_clone);
    }
}
