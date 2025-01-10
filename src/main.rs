use std::{
    collections::HashMap,
    io::Error,
    io::ErrorKind,
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
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Init {
    /// The path at which to create the config file.
    #[arg(short, long, default_value = ".daft.toml")]
    path: PathBuf,
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct ConfigPath {
    /// Path to configuration file.
    #[arg(short, long, default_value = ".daft.toml")]
    path: PathBuf,
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
    8
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
}

#[derive(Default, Debug, Serialize, Clone, PartialEq, Eq)]
struct RayProvider {
    r#type: StrRef,
    region: StrRef,
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
    iam_instance_profile: IamInstanceProfile,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
struct IamInstanceProfile {
    name: Option<StrRef>,
    arn: Option<StrRef>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
struct RayResources {
    cpu: usize,
}

async fn read_and_convert(daft_config_path: &Path) -> anyhow::Result<RayConfig> {
    fn convert(daft_config: DaftConfig) -> RayConfig {
        RayConfig {
            cluster_name: daft_config.setup.name,
            max_workers: daft_config.setup.number_of_workers,
            provider: RayProvider {
                r#type: "aws".into(),
                region: "us-west-2".into(),
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
                        node_config: RayNodeConfig {
                            key_name: "a".into(),
                            instance_type: daft_config.setup.instance_type.clone(),
                            image_id: daft_config.setup.image_id.clone(),
                            iam_instance_profile: IamInstanceProfile {
                                name: daft_config.setup.iam_instance_profile.name.clone(),
                                arn: daft_config.setup.iam_instance_profile.arn.clone(),
                            },
                        },
                        resources: Some(RayResources { cpu: 0 }),
                    },
                ),
                (
                    "ray.worker.default".into(),
                    RayNodeType {
                        max_workers: daft_config.setup.number_of_workers,
                        node_config: RayNodeConfig {
                            key_name: "a".into(),
                            instance_type: daft_config.setup.instance_type,
                            image_id: daft_config.setup.image_id,
                            iam_instance_profile: IamInstanceProfile {
                                name: daft_config.setup.iam_instance_profile.name,
                                arn: daft_config.setup.iam_instance_profile.arn,
                            },
                        },
                        resources: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        }
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
    let ray_config = convert(daft_config);

    Ok(ray_config)
}

async fn write_ray_config(ray_config: RayConfig, dest: impl AsRef<Path>) -> anyhow::Result<()> {
    let ray_config = serde_yaml::to_string(&ray_config)?;
    fs::write(dest, ray_config).await?;
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
        SubCommand::Check(ConfigPath { path }) => {
            let _ = read_and_convert(&path).await?;
        }
        SubCommand::Export(ConfigPath { path }) => {
            let ray_config = read_and_convert(&path).await?;
            write_ray_config(ray_config, ".ray.yaml").await?;
        }
        SubCommand::Up(ConfigPath { path }) => {
            let temp_dir = TempDir::new("daft-launcher")?;
            let mut ray_path = temp_dir.path().to_owned();
            ray_path.push("ray.yaml");
            let ray_config = read_and_convert(&path).await?;
            write_ray_config(ray_config, ray_path).await?;
            // Command::new("ray").args(["up"]);
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
                path: ".daft.toml".into(),
            }),
            verbosity: 0,
        })
        .await
        .unwrap();
    }
}
