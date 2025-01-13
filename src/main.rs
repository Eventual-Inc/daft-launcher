use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(not(test))]
use anyhow::bail;
use clap::{Parser, Subcommand};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use tempdir::TempDir;
use tokio::{fs, process::Command};

type StrRef = Arc<str>;
type PathRef = Arc<Path>;

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct DaftLauncher {
    #[command(subcommand)]
    sub_command: SubCommand,

    /// Enable verbose printing
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbosity: u8,
}

#[derive(Debug, Subcommand, Clone, PartialEq, Eq)]
enum SubCommand {
    /// Initialize a daft-launcher configuration file.
    ///
    /// If no path is provided, this will create a default ".daft.toml" in the
    /// current working directory.
    Init(Init),

    /// Check to make sure the daft-launcher configuration file is correct.
    Check(ConfigPath),

    /// Export the daft-launcher configuration file to a ray configuration file.
    Export(ConfigPath),

    /// Spin up a new cluster.
    Up(ConfigPath),

    /// Spin down a given cluster and put the nodes to "sleep".
    ///
    /// This will *not* delete the nodes, only stop them. The nodes can be
    /// restarted at a future time.
    Stop(ConfigPath),

    /// Spin down a given cluster and fully terminate the nodes.
    ///
    /// This *will* delete the nodes; they will not be accessible from here on
    /// out.
    Kill(ConfigPath),
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Init {
    /// The path at which to create the config file.
    #[arg(default_value = ".daft.toml")]
    path: PathBuf,
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct ConfigPath {
    /// Path to configuration file.
    #[arg(default_value = ".daft.toml")]
    config: PathBuf,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
struct DaftConfig {
    setup: DaftSetup,
    #[serde(default)]
    run: DaftRun,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
struct DaftSetup {
    name: StrRef,
    #[serde(deserialize_with = "parse_version_req")]
    version: VersionReq,
    provider: DaftProvider,
    region: StrRef,
    #[serde(default = "default_number_of_workers")]
    number_of_workers: usize,
    ssh_user: StrRef,
    #[serde(deserialize_with = "parse_ssh_private_key")]
    ssh_private_key: PathRef,
    #[serde(default = "default_instance_type")]
    instance_type: StrRef,
    #[serde(default = "default_image_id")]
    image_id: StrRef,
    #[serde(default)]
    iam_instance_profile: IamInstanceProfile,
    #[serde(default)]
    dependencies: Vec<StrRef>,
}

fn parse_ssh_private_key<'de, D>(deserializer: D) -> Result<PathRef, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let path: PathRef = Deserialize::deserialize(deserializer)?;
    let path = if path.starts_with("~") {
        let mut home = PathBuf::from(env!("HOME"));
        for segment in path.into_iter().skip(1) {
            home.push(segment);
        }
        Arc::from(home)
    } else {
        path
    };

    #[cfg(test)]
    {
        Ok(path)
    }

    #[cfg(not(test))]
    {
        if path.exists() {
            Ok(path)
        } else {
            Err(serde::de::Error::custom(format!(
                "The path, {path:?}, does not exist"
            )))
        }
    }
}

fn default_number_of_workers() -> usize {
    4
}

fn default_instance_type() -> StrRef {
    "i3.2xlarge".into()
}

fn default_image_id() -> StrRef {
    "ami-04dd23e62ed049936".into()
}

fn parse_version_req<'de, D>(deserializer: D) -> Result<VersionReq, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: StrRef = Deserialize::deserialize(deserializer)?;
    let version_req = raw
        .parse::<VersionReq>()
        .map_err(serde::de::Error::custom)?;
    let current_version = env!("CARGO_PKG_VERSION").parse::<Version>().unwrap();
    if version_req.matches(&current_version) {
        Ok(version_req)
    } else {
        Err(serde::de::Error::custom(format!("You're running daft-launcher version {current_version}, but your configuration file requires version {version_req}")))
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum DaftProvider {
    Aws,
}

#[derive(Default, Debug, Deserialize, Clone, PartialEq, Eq)]
struct DaftRun {
    #[serde(default)]
    pre_setup_commands: Vec<StrRef>,
    #[serde(default)]
    post_setup_commands: Vec<StrRef>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct RayConfig {
    cluster_name: StrRef,
    max_workers: usize,
    provider: RayProvider,
    auth: RayAuth,
    available_node_types: HashMap<StrRef, RayNodeType>,
    setup_commands: Vec<StrRef>,
}

#[derive(Default, Debug, Serialize, Clone, PartialEq, Eq)]
struct RayProvider {
    r#type: StrRef,
    region: StrRef,
    cache_stopped_nodes: bool,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct RayAuth {
    ssh_user: StrRef,
    ssh_private_key: PathRef,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct RayNodeType {
    max_workers: usize,
    node_config: RayNodeConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<RayResources>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
struct RayNodeConfig {
    key_name: StrRef,
    instance_type: StrRef,
    image_id: StrRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    iam_instance_profile: Option<IamInstanceProfile>,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
struct IamInstanceProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<StrRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arn: Option<StrRef>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
struct RayResources {
    cpu: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TeardownBehaviour {
    Stop,
    Kill,
}

async fn read_and_convert(
    daft_config_path: &Path,
    teardown_behaviour: Option<TeardownBehaviour>,
) -> anyhow::Result<RayConfig> {
    fn convert(
        daft_config: DaftConfig,
        teardown_behaviour: Option<TeardownBehaviour>,
    ) -> anyhow::Result<RayConfig> {
        let key_name = daft_config
            .setup
            .ssh_private_key
            .file_stem()
            .ok_or_else(|| anyhow::anyhow!(""))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!(""))?
            .into();
        let iam_instance_profile = if daft_config.setup.iam_instance_profile.name.is_some()
            || daft_config.setup.iam_instance_profile.arn.is_some()
        {
            Some(IamInstanceProfile {
                name: daft_config.setup.iam_instance_profile.name.clone(),
                arn: daft_config.setup.iam_instance_profile.arn.clone(),
            })
        } else {
            None
        };
        let node_config = RayNodeConfig {
            key_name,
            instance_type: daft_config.setup.instance_type.clone(),
            image_id: daft_config.setup.image_id.clone(),
            iam_instance_profile,
        };
        Ok(RayConfig {
            cluster_name: daft_config.setup.name,
            max_workers: daft_config.setup.number_of_workers,
            provider: RayProvider {
                r#type: "aws".into(),
                region: "us-west-2".into(),
                cache_stopped_nodes: match teardown_behaviour {
                    Some(TeardownBehaviour::Stop) | None => true,
                    Some(TeardownBehaviour::Kill) => false,
                },
            },
            auth: RayAuth {
                ssh_user: daft_config.setup.ssh_user,
                ssh_private_key: daft_config.setup.ssh_private_key,
            },
            available_node_types: vec![
                (
                    "ray.head.default".into(),
                    RayNodeType {
                        max_workers: daft_config.setup.number_of_workers,
                        node_config: node_config.clone(),
                        resources: Some(RayResources { cpu: 0 }),
                    },
                ),
                (
                    "ray.worker.default".into(),
                    RayNodeType {
                        max_workers: daft_config.setup.number_of_workers,
                        node_config,
                        resources: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            setup_commands: {
                let mut commands = vec![
                    "curl -LsSf https://astral.sh/uv/install.sh | sh".into(),
                    "uv python install 3.12".into(),
                    "uv python pin 3.12".into(),
                    "uv venv".into(),
                    "echo 'source $HOME/.venv/bin/activate' >> ~/.bashrc".into(),
                    "source ~/.bashrc".into(),
                    "uv pip install boto3 pip ray[default] getdaft py-spy deltalake".into(),
                ];
                if !daft_config.setup.dependencies.is_empty() {
                    let deps = daft_config
                        .setup
                        .dependencies
                        .iter()
                        .map(|dep| format!(r#""{dep}""#))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let deps = format!("uv pip install {deps}").into();
                    commands.push(deps);
                }
                commands
            },
        })
    }

    let contents = fs::read_to_string(&daft_config_path)
        .await
        .map_err(|error| {
            if let ErrorKind::NotFound = error.kind() {
                Error::new(
                    ErrorKind::NotFound,
                    format!("The file {daft_config_path:?} does not exist"),
                )
            } else {
                error
            }
        })?;
    let daft_config = toml::from_str::<DaftConfig>(&contents)?;
    let ray_config = convert(daft_config, teardown_behaviour)?;

    Ok(ray_config)
}

async fn write_ray_config(ray_config: RayConfig, dest: impl AsRef<Path>) -> anyhow::Result<()> {
    let ray_config = serde_yaml::to_string(&ray_config)?;
    fs::write(dest, ray_config).await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpinDirection {
    Up,
    Down,
}

impl SpinDirection {
    fn as_str(&self) -> &str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
        }
    }
}

async fn manage_cluster(
    daft_config_path: &Path,
    spin_direction: SpinDirection,
    teardown_behaviour: Option<TeardownBehaviour>,
) -> anyhow::Result<()> {
    let temp_dir = TempDir::new("daft-launcher")?;
    let mut ray_path = temp_dir.path().to_owned();
    ray_path.push("ray.yaml");
    let ray_config = read_and_convert(daft_config_path, teardown_behaviour).await?;
    write_ray_config(ray_config, &ray_path).await?;
    let _ = Command::new("ray")
        .arg(spin_direction.as_str())
        .arg(ray_path)
        .arg("-y")
        .spawn()?
        .wait_with_output()
        .await?;
    Ok(())
}

async fn run(daft_launcher: DaftLauncher) -> anyhow::Result<()> {
    match daft_launcher.sub_command {
        SubCommand::Init(Init { path }) => {
            #[cfg(not(test))]
            if path.exists() {
                bail!("The path {path:?} already exists; the path given must point to a new location on your filesystem");
            }
            let contents = include_str!("template.toml");
            let contents = contents.replace("<VERSION>", env!("CARGO_PKG_VERSION"));
            fs::write(path, contents).await?;
        }
        SubCommand::Check(ConfigPath { config: path }) => {
            let _ = read_and_convert(&path, None).await?;
        }
        SubCommand::Export(ConfigPath { config: path }) => {
            let ray_path = PathBuf::from(".ray.yaml");
            #[cfg(not(test))]
            if ray_path.exists() {
                bail!("The file {ray_path:?} already exists; delete it before writing new configurations to that file");
            }
            let ray_config = read_and_convert(&path, None).await?;
            write_ray_config(ray_config, ray_path).await?;
        }
        SubCommand::Up(ConfigPath { config: path }) => {
            manage_cluster(&path, SpinDirection::Up, None).await?
        }
        SubCommand::Stop(ConfigPath { config: path }) => {
            manage_cluster(&path, SpinDirection::Down, Some(TeardownBehaviour::Stop)).await?
        }
        SubCommand::Kill(ConfigPath { config: path }) => {
            manage_cluster(&path, SpinDirection::Down, Some(TeardownBehaviour::Kill)).await?
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run(DaftLauncher::parse()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_and_export() {
        run(DaftLauncher {
            sub_command: SubCommand::Init(Init {
                path: ".daft.toml".into(),
            }),
            verbosity: 0,
        })
        .await
        .unwrap();
        run(DaftLauncher {
            sub_command: SubCommand::Check(ConfigPath {
                config: ".daft.toml".into(),
            }),
            verbosity: 0,
        })
        .await
        .unwrap();
    }
}
