use tempdir::TempDir;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::{
    config::ray::RayConfig,
    utils::{create_temporary_ray_file, path_to_str},
    PathRef,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaySubcommand {
    Up,
    Down,
    Submit,
}

impl AsRef<str> for RaySubcommand {
    fn as_ref(&self) -> &str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Submit => "submit",
        }
    }
}

pub async fn run_ray(
    subcommand: RaySubcommand,
    ray_config: &RayConfig,
    args: &[&str],
) -> anyhow::Result<Vec<u8>> {
    const RAY_CMD: &str = "ray";
    const PYTHON_BUFFERING_ENV_VAR: &str = "PYTHONUNBUFFERED";
    const PYTHON_BUFFERING_ENV_VALUE: &str = "1";

    let (temp_dir, path) = write_ray(&ray_config).await?;

    let child = Command::new(RAY_CMD)
        .env(PYTHON_BUFFERING_ENV_VAR, PYTHON_BUFFERING_ENV_VALUE)
        .arg(subcommand.as_ref())
        .arg(path_to_str(path.as_os_str())?)
        .args(args)
        .arg("-y")
        .spawn()?;

    let output = child.wait_with_output().await?;

    // Explicitly deletes the entire temporary directory.
    // The config file that we wrote to inside of there will now be deleted.
    //
    // This should only happen *after* the `ray` command has finished executing.
    drop(temp_dir);

    if output.status.success() {
        Ok(output.stdout)
    } else {
        anyhow::bail!("Command failed with exit status: {}", output.status)
    }
}

pub async fn write_ray(ray_config: &RayConfig) -> anyhow::Result<(TempDir, PathRef)> {
    let contents =
        serde_yaml::to_string(ray_config).expect("Serialization to yaml should always succeed");
    let (temp_dir, path, mut file) = create_temporary_ray_file().await?;

    log::debug!("Writing ray config to temporary file {}", path.display());
    log::debug!("{}", contents);
    file.write_all(contents.as_bytes()).await?;

    Ok((temp_dir, path))
}
