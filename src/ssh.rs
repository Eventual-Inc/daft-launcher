use std::{net::Ipv4Addr, path::Path, process::Stdio, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    time::timeout,
};

use crate::AwsConfig;

async fn get_head_node_ip(ray_path: impl AsRef<Path>) -> anyhow::Result<Ipv4Addr> {
    let mut ray_command = Command::new("ray")
        .arg("get-head-ip")
        .arg(ray_path.as_ref())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut tail_command = Command::new("tail")
        .args(["-n", "1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut writer = tail_command.stdin.take().expect("stdin must exist");

    tokio::spawn(async move {
        let mut reader = BufReader::new(ray_command.stdout.take().expect("stdout must exist"));
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).await?;
        writer.write_all(&buffer).await?;
        Ok::<_, anyhow::Error>(())
    });
    let output = tail_command.wait_with_output().await?;
    if !output.status.success() {
        anyhow::bail!("Failed to fetch ip address of head node");
    };
    let addr = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<Ipv4Addr>()?;
    Ok(addr)
}

async fn generate_ssh_command(
    ray_path: impl AsRef<Path>,
    aws_config: &AwsConfig,
    portforward: Option<u16>,
    verbose: bool,
) -> anyhow::Result<(Ipv4Addr, Command)> {
    // match &daft_config.setup.provider_config {
    //     ProviderConfig::Aws(aws_config) => {
    //     }
    //     ProviderConfig::K8s(..) => todo!(),
    // }
    let user = aws_config.ssh_user.as_ref();
    let addr = get_head_node_ip(ray_path).await?;

    let mut command = Command::new("ssh");

    command
        .arg("-i")
        .arg(aws_config.ssh_private_key.as_ref())
        .arg("-o")
        .arg("StrictHostKeyChecking=no");

    if let Some(portforward) = portforward {
        command
            .arg("-N")
            .arg("-L")
            .arg(format!("{portforward}:localhost:8265"));
    };

    if verbose {
        command.arg("-v");
    }

    command.arg(format!("{user}@{addr}")).kill_on_drop(true);

    Ok((addr, command))
}

pub async fn ssh(ray_path: impl AsRef<Path>, aws_config: &AwsConfig) -> anyhow::Result<()> {
    let (_, mut command) = generate_ssh_command(ray_path, aws_config, None, false).await?;
    let exit_status = command.spawn()?.wait().await?;
    if exit_status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to ssh into the ray cluster"))
    }
}

pub async fn ssh_portforward(
    ray_path: impl AsRef<Path>,
    aws_config: &AwsConfig,
    portforward: Option<u16>,
) -> anyhow::Result<Child> {
    let (addr, mut command) = generate_ssh_command(
        ray_path,
        aws_config,
        Some(portforward.unwrap_or(8265)),
        true,
    )
    .await?;
    let mut child = command.stderr(Stdio::piped()).spawn()?;

    // We wait for the ssh port-forwarding process to write a specific string to the
    // output.
    //
    // This is a little hacky (and maybe even incorrect across platforms) since we
    // are just parsing the output and observing if a specific string has been
    // printed. It may be incorrect across platforms because the SSH standard
    // does *not* specify a standard "success-message" to printout if the ssh
    // port-forward was successful.
    timeout(Duration::from_secs(5), {
        let stderr = child.stderr.take().expect("stderr must exist");
        async move {
            let mut lines = BufReader::new(stderr).lines();
            loop {
                let Some(line) = lines.next_line().await? else {
                    anyhow::bail!("Failed to establish ssh port-forward to {addr}");
                };
                if line.starts_with(format!("Authenticated to {addr}").as_str()) {
                    break Ok(());
                }
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Establishing an ssh port-forward to {addr} timed out"))??;

    Ok(child)
}
