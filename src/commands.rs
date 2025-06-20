use crate::session::{AppState, LoginStatus, Session, User};
use anyhow::{Context, Result};

pub trait Command {
    fn name(&self) -> &str;
    async fn execute(&self, session: Session) -> Result<LoginStatus>;
    fn help(&self) -> String;
}

pub struct Login;

impl Command for Login {
    fn name(&self) -> &str {
        "login"
    }

    async fn execute(&self, mut session: Session) -> Result<LoginStatus> {
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

        Ok(session.login_status)
    }

    fn help(&self) -> String {
        todo!()
    }
}
