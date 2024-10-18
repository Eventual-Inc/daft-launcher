use std::{io::Write, process::Command};

use clap::Parser;
use tempdir::TempDir;

use crate::{
    cli::{Cli, Connect, Dashboard, Down, InitConfig, Sql, Submit, Up},
    config::{read_custom, write_ray, write_ray_cluster_name},
    utils::{assert_is_authenticated_with_aws, create_new_file, path_to_str},
    PathRef,
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

fn run_ray_command(
    temp_dir: TempDir,
    path: PathRef,
    sub_command: &str,
    args: Option<&[&str]>,
) -> anyhow::Result<()> {
    let args = args.unwrap_or(&[]);
    let _ = Command::new("ray")
        .arg(sub_command)
        .arg(path_to_str(path.as_os_str())?)
        .args(args)
        .spawn()?
        .wait()?;

    // Explicitly deletes the entire temporary directory.
    // The config file that we wrote to inside of there will now be deleted.
    drop(temp_dir);

    Ok(())
}

fn handle_up(up: Up) -> anyhow::Result<()> {
    let (_, ray_config) = read_custom(&up.config.config)?;
    let (temp_dir, path) = write_ray(&ray_config)?;
    run_ray_command(temp_dir, path, "up", None)
}

fn handle_down(down: Down) -> anyhow::Result<()> {
    match (down.name, down.r#type, down.region) {
        (Some(name), Some(r#type), Some(region)) => {
            let (temp_dir, path) =
                write_ray_cluster_name(&name, &r#type, &region)?;
            run_ray_command(temp_dir, path, "down", None)
        }
        (None, None, None) => {
            let (_, ray_config) = read_custom(&down.config.config)?;
            let (temp_dir, path) = write_ray(&ray_config)?;
            run_ray_command(temp_dir, path, "down", None)
        }
        _ => anyhow::bail!("You must provide all three arguments to spin down a cluster: name, type, and region"),
    }
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
