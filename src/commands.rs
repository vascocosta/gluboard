use crate::session::{self, AppStateKind, LoginStatus, Message, Session, User};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bcrypt::DEFAULT_COST;
use std::collections::HashMap;

pub struct CommandHandler {
    welcome_commands: HashMap<&'static str, Box<dyn Command + Send + Sync>>,
    message_commands: HashMap<&'static str, Box<dyn Command + Send + Sync>>,
}

impl CommandHandler {
    pub fn new() -> Self {
        let mut welcome_commands: HashMap<&'static str, Box<dyn Command + Send + Sync>> =
            HashMap::new();

        welcome_commands.insert("login", Box::new(Login));
        welcome_commands.insert("register", Box::new(Register));

        let mut message_commands: HashMap<&'static str, Box<dyn Command + Send + Sync>> =
            HashMap::new();

        message_commands.insert("message", Box::new(Messages));

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
    fn name(&self) -> &str;
    async fn execute(&self, session: &mut Session, args: Option<&[&str]>) -> Result<()>;
    fn help(&self) -> String;
}

pub struct Login;

#[async_trait]
impl Command for Login {
    fn name(&self) -> &str {
        "login"
    }

    async fn execute(&self, session: &mut Session, args: Option<&[&str]>) -> Result<()> {
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

pub struct Register;

#[async_trait]
impl Command for Register {
    fn name(&self) -> &str {
        "register"
    }

    async fn execute(&self, session: &mut Session, args: Option<&[&str]>) -> Result<()> {
        let username = session.prompt("Choose a username: ").await?;
        let password = session.prompt("Choose a password: ").await?;

        let user = User {
            id: 1,
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

pub struct Messages;

#[async_trait]
impl Command for Messages {
    fn name(&self) -> &str {
        "message"
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
                    let body = session.prompt("Body: ").await?;

                    let username = match &session.login_status {
                        LoginStatus::Success(username) => username,
                        LoginStatus::Failure => todo!(),
                    };

                    let message = Message {
                        id: 0,
                        username: username.to_owned(),
                        subject,
                        body,
                    };

                    session.app_state.messages.write().await.push(message);
                    session.app_state.save(AppStateKind::Messages).await?;

                    Ok(())
                }
                _ => session.writeln("Unknown sub command").await,
            },
            Some([sub_command, sub_arg]) => {
                session.writeln(&format!("{sub_command} {sub_arg}")).await
            }
            Some(&[]) | Some(&[_, _, _, ..]) => session.writeln("Show usage").await,
        }
    }

    fn help(&self) -> String {
        todo!()
    }
}
