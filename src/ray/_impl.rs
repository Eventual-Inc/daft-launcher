use std::process::Stdio;

use console::style;
use futures::{Stream, StreamExt};
use tempdir::TempDir;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::Command,
};

use crate::{
    config::ray::RayConfig,
    utils::{create_temporary_ray_file, is_debug, path_to_str},
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
    path: PathRef,
    args: &[&str],
    print_helper: impl 'static + Fn(&str) + Send,
) -> anyhow::Result<()> {
    const RAY_CMD: &str = "ray";
    const PYTHON_BUFFERING_ENV_VAR: &str = "PYTHONUNBUFFERED";
    const PYTHON_BUFFERING_ENV_VALUE: &str = "1";

    let mut child = Command::new(RAY_CMD)
        .env(PYTHON_BUFFERING_ENV_VAR, PYTHON_BUFFERING_ENV_VALUE)
        .arg(subcommand.as_ref())
        .arg("-y")
        .arg(path_to_str(path.as_os_str())?)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let child_stdout = BufReader::new(child.stdout.take().expect("Stdout should always exist"));
    to_async_iter(child_stdout.lines())
        .for_each(|line| {
            let print_helper = &print_helper;
            async move { print_helper(&line) }
        })
        .await;

    let exit_status = child.wait().await?;

    if exit_status.success() {
        Ok(())
    } else {
        let child_stderr = BufReader::new(child.stderr.take().expect("Stderr should always exist"));

        if is_debug() {
            let full_child_backtrace = to_async_iter(child_stderr.lines())
                .fold(String::default(), |mut acc, line| async move {
                    acc.push_str(&line);
                    acc
                })
                .await;
            anyhow::bail!(
                "Command failed with exit status: {}\n{}",
                exit_status,
                full_child_backtrace,
            )
        } else {
            let last_line = to_async_iter(child_stderr.lines())
                .fold(None, |_, line| async move { Some(line) })
                .await;
            match last_line {
                Some(last_line) => anyhow::bail!(
                    "Command failed with exit status: {}\nReason: {}",
                    exit_status,
                    style(last_line).red().dim(),
                ),
                None => anyhow::bail!("Command failed with exit status: {}", exit_status),
            }
        }
    }
}

fn to_async_iter<I: Unpin + tokio::io::AsyncBufRead>(
    lines: Lines<I>,
) -> impl Stream<Item = String> {
    futures::stream::unfold(lines, |mut lines| async {
        lines
            .next_line()
            .await
            .expect("Reading line from child process should always succeed")
            .map(|line| (line, lines))
    })
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
