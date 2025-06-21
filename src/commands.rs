use crate::session::{LoginStatus, Session, User};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bcrypt::DEFAULT_COST;
use std::collections::HashMap;

pub struct CommandHandler {
    commands: HashMap<&'static str, Box<dyn Command + Send + Sync>>,
}

impl CommandHandler {
    pub fn new() -> Self {
        let mut commands: HashMap<&'static str, Box<dyn Command + Send + Sync>> = HashMap::new();

        commands.insert("login", Box::new(Login));
        commands.insert("register", Box::new(Register));

        Self { commands }
    }

    pub async fn handle(&self, raw_command: &str, session: &mut Session) -> Result<()> {
        let name = raw_command
            .split_whitespace()
            .next()
            .context("Invalid command")?;

        self.commands
            .get(name)
            .context("Unknown command")?
            .execute(session)
            .await
    }
}

#[allow(dead_code)]
#[async_trait]
pub trait Command {
    fn name(&self) -> &str;
    async fn execute(&self, session: &mut Session) -> Result<()>;
    fn help(&self) -> String;
}

pub struct Login;

#[async_trait]
impl Command for Login {
    fn name(&self) -> &str {
        "login"
    }

    async fn execute(&self, session: &mut Session) -> Result<()> {
        loop {
            let username = session.prompt("Username: ").await?;
            let password = session.prompt("Password: ").await?;

            let users = session.app_state.users.read().await;
            let user: &User = users
                .iter()
                .filter(|u| u.username == username)
                .collect::<Vec<&User>>()
                .first()
                .context("Could not find user")?;

            let valid_password = bcrypt::verify(password, &user.password)?;

            if !valid_password {
                session.login_status = LoginStatus::Failure;
                // session.writeln("Login failed!").await?;
            } else {
                session.login_status = LoginStatus::Success(user.username.clone());
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

    async fn execute(&self, session: &mut Session) -> Result<()> {
        let username = session.prompt("Choose a username: ").await?;
        let password = session.prompt("Choose a password: ").await?;

        let user = User {
            id: 1,
            username: username.to_owned(),
            password: bcrypt::hash(password, DEFAULT_COST)?,
        };

        session.app_state.users.write().await.push(user);
        session.app_state.save().await?;

        Ok(())
    }

    fn help(&self) -> String {
        todo!()
    }
}
