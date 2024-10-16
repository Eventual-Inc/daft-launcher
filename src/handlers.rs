use std::{io::Write, process::Command};

use clap::Parser;

use crate::{
    cli::{Cli, Connect, Dashboard, Down, InitConfig, Sql, Submit, Up},
    config::{read_custom, write_ray},
    utils::{assert_is_authenticated_with_aws, create_new_file},
};

const DEFAULT_CONFIG: &str =
    include_str!(path_from_root!("assets" / "default.toml"));

pub async fn handle() -> anyhow::Result<()> {
    match Cli::parse() {
        Cli::InitConfig(init_config) => handle_init_config(init_config),
        Cli::Up(up) => {
            assert_is_authenticated_with_aws().await?;
            handle_up(up)
        }
        Cli::Down(down) => {
            assert_is_authenticated_with_aws().await?;
            handle_down(down)
        }
        Cli::Submit(submit) => handle_submit(submit),
        Cli::Connect(connect) => handle_connect(connect),
        Cli::Dashboard(dashboard) => handle_dashboard(dashboard),
        Cli::Sql(sql) => handle_sql(sql),
    }
}

fn handle_init_config(init_config: InitConfig) -> anyhow::Result<()> {
    if init_config.interactive {
        todo!()
    } else {
        create_new_file(&init_config.name)?
            .write_all(DEFAULT_CONFIG.as_bytes())?;
        Ok(())
    }
}

fn handle_up(up: Up) -> anyhow::Result<()> {
    let custom_config = read_custom(&up.config.config)?;
    let ray_config = custom_config.try_into()?;
    let (temp_dir, path) = write_ray(&ray_config)?;
    let _ = Command::new("ray")
        .args([
            "up",
            path.to_str().expect("Invalid characters in file path"),
        ])
        .spawn()?
        .wait()?;

    // Explicitly deletes the entire temporary directory.
    // The config file that we wrote to inside of there will now be deleted.
    drop(temp_dir);

    Ok(())
}

fn handle_down(_: Down) -> anyhow::Result<()> {
    todo!()
}

fn handle_submit(_: Submit) -> anyhow::Result<()> {
    todo!()
}

fn handle_connect(_: Connect) -> anyhow::Result<()> {
    todo!()
}

fn handle_dashboard(_: Dashboard) -> anyhow::Result<()> {
    todo!()
}

fn handle_sql(_: Sql) -> anyhow::Result<()> {
    todo!()
}
