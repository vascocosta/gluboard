use crate::session::{AppState, ConnectionManager, LoginStatus, User};
use anyhow::{Context, Result};
use tokio::{io::BufReader, net::TcpStream};

pub trait Command {
    fn name(&self) -> &str;
    async fn execute(&self, ctx: CommandContext) -> Result<LoginStatus>;
    fn help(&self) -> String;
}

pub struct CommandContext {
    pub app_state: AppState,
    pub connection_manager: ConnectionManager,
}

pub struct Login;

impl Command for Login {
    fn name(&self) -> &str {
        "login"
    }

    async fn execute(&self, mut ctx: CommandContext) -> Result<LoginStatus> {
        let username = ctx.connection_manager.prompt("Username: ").await?;
        let password = ctx.connection_manager.prompt("Password: ").await?;

        let users = ctx.app_state.users.read().await;
        let user: &User = users
            .iter()
            .filter(|u| u.username == username)
            .collect::<Vec<&User>>()
            .first()
            .context("Could not find user")?;

        let valid_password = bcrypt::verify(password, &user.password)?;

        if !valid_password {
            ctx.connection_manager.login_status = LoginStatus::Failure;
        } else {
            ctx.connection_manager.login_status = LoginStatus::Success(user.username.clone());
        }

        Ok(ctx.connection_manager.login_status)
    }

    fn help(&self) -> String {
        todo!()
    }
}
