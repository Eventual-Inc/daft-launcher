use std::{io::Write, process};

use clap::Parser;

use crate::{cli, utils};

const DEFAULT_CONFIG: &str = include_str!(path_from_root!("assets" / "default.toml"));

pub async fn handle() -> anyhow::Result<()> {
    match cli::Cli::parse() {
        cli::Cli::InitConfig(init_config) => handle_init_config(init_config),
        cli::Cli::Up(up) => {
            utils::assert_is_authenticated_with_aws().await?;
            handle_up(up)
        }
        cli::Cli::Down(down) => {
            utils::assert_is_authenticated_with_aws().await?;
            handle_down(down)
        }
        cli::Cli::Submit(submit) => handle_submit(submit),
        cli::Cli::Connect(connect) => handle_connect(connect),
        cli::Cli::Dashboard(dashboard) => handle_dashboard(dashboard),
        cli::Cli::Sql(sql) => handle_sql(sql),
    }
}

fn handle_init_config(init_config: cli::InitConfig) -> anyhow::Result<()> {
    if init_config.interactive {
        todo!()
    } else {
        utils::create_new_file(&init_config.name)?.write_all(DEFAULT_CONFIG.as_bytes())?;
        Ok(())
    }
}

fn handle_up(up: cli::Up) -> anyhow::Result<()> {
    let custom_config = utils::read_custom_config(&up.config.config)?;
    let ray_config = custom_config.try_into()?;
    let (temp_dir, path) = utils::write_ray_config(&ray_config)?;
    let _ = process::Command::new("ray")
        .args(["up", path.to_str().expect("Invalid characters in file")])
        .spawn()?
        .wait()?;
    drop(temp_dir);
    Ok(())
}

fn handle_down(_: cli::Down) -> anyhow::Result<()> {
    todo!()
}

fn handle_submit(_: cli::Submit) -> anyhow::Result<()> {
    todo!()
}

fn handle_connect(_: cli::Connect) -> anyhow::Result<()> {
    todo!()
}

fn handle_dashboard(_: cli::Dashboard) -> anyhow::Result<()> {
    todo!()
}

fn handle_sql(_: cli::Sql) -> anyhow::Result<()> {
    todo!()
}
