use crate::session::{LoginStatus, Session, User};
use anyhow::{Context, Result};
use bcrypt::DEFAULT_COST;

#[allow(dead_code)]
pub trait Command {
    fn name(&self) -> &str;
    async fn execute(&self, session: &mut Session) -> Result<()>;
    fn help(&self) -> String;
}

pub struct Login;

impl Command for Login {
    fn name(&self) -> &str {
        "login"
    }

    async fn execute(&self, session: &mut Session) -> Result<()> {
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
        } else {
            session.login_status = LoginStatus::Success(user.username.clone());
        }

        Ok(())
    }

    fn help(&self) -> String {
        todo!()
    }
}

pub struct Register;

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

        Ok(())
    }

    fn help(&self) -> String {
        todo!()
    }
}
