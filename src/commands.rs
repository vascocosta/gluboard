use crate::session::{AppState, LoginStatus, Session, User};
use anyhow::{Context, Result};

pub trait Command {
    fn name(&self) -> &str;
    async fn execute(&self, ctx: CommandContext) -> Result<LoginStatus>;
    fn help(&self) -> String;
}

pub struct CommandContext {
    pub app_state: AppState,
    pub session: Session,
}

pub struct Login;

impl Command for Login {
    fn name(&self) -> &str {
        "login"
    }

    async fn execute(&self, mut ctx: CommandContext) -> Result<LoginStatus> {
        let username = ctx.session.prompt("Username: ").await?;
        let password = ctx.session.prompt("Password: ").await?;

        let users = ctx.app_state.users.read().await;
        let user: &User = users
            .iter()
            .filter(|u| u.username == username)
            .collect::<Vec<&User>>()
            .first()
            .context("Could not find user")?;

        let valid_password = bcrypt::verify(password, &user.password)?;

        if !valid_password {
            ctx.session.login_status = LoginStatus::Failure;
        } else {
            ctx.session.login_status = LoginStatus::Success(user.username.clone());
        }

        Ok(ctx.session.login_status)
    }

    fn help(&self) -> String {
        todo!()
    }
}
