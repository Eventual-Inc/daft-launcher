use tempdir::TempDir;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::{
    config::{raw::Job, ray::RayConfig},
    utils::{create_temporary_ray_file, path_to_str},
    PathRef,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RayCommand {
    Up,
    Down,
    Job(RayJob),
}

impl AsRef<[&'static str]> for RayCommand {
    fn as_ref(&self) -> &[&'static str] {
        match self {
            Self::Up => &["up"],
            Self::Down => &["down"],
            Self::Job(RayJob::Submit(..)) => &["job", "submit"],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RayJob {
    Submit(Job),
}

pub async fn run_ray(ray_command: RayCommand, ray_config: &RayConfig) -> anyhow::Result<Vec<u8>> {
    const RAY_CMD: &str = "ray";
    const PYTHON_BUFFERING_ENV_VAR: &str = "PYTHONUNBUFFERED";
    const PYTHON_BUFFERING_ENV_VALUE: &str = "1";

    let mut command = Command::new(RAY_CMD);
    command
        .env(PYTHON_BUFFERING_ENV_VAR, PYTHON_BUFFERING_ENV_VALUE)
        .args(ray_command.as_ref());

    let output = match ray_command {
        RayCommand::Up | RayCommand::Down => {
            let (temp_dir, path) = write_ray(&ray_config).await?;

            let child = command
                .arg(path_to_str(path.as_os_str())?)
                .arg("-y")
                .spawn()?;

            let output = child.wait_with_output().await?;

            // Explicitly deletes the entire temporary directory.
            // The config file that we wrote to inside of there will now be deleted.
            //
            // This should only happen *after* the `ray` command has finished executing.
            drop(temp_dir);

            output
        }
        RayCommand::Job(RayJob::Submit(job)) => {
            command
                .arg("--working-dir")
                .arg(&*job.working_dir)
                .arg("--address")
                .arg("http://localhost:8265")
                .arg("--")
                .args(job.command.split(' '))
                .spawn()?
                .wait_with_output()
                .await?
        }
    };

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
