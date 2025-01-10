use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::bail;
use clap::{Parser, Subcommand};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

type StrRef = Arc<str>;
type PathRef = Arc<Path>;

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Command {
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

    /// Spin up a new cluster.
    ///
    /// If no configuration file path is provided, this will try to find a
    /// ".daft.toml" in the current working directory and use that.
    Up(Up),
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Init {
    /// The path at which to create the config file.
    #[arg(short, long, default_value = ".daft.toml")]
    path: PathBuf,
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Up {
    #[clap(flatten)]
    config: ConfigPath,
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
    iam_instance_profile_arn: Option<StrRef>,
    instance_type: Option<StrRef>,
    image_id: Option<StrRef>,
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

    if path.exists() {
        Ok(path)
    } else {
        Err(serde::de::Error::custom(format!(
            "The path, {path:?}, does not exist"
        )))
    }
}

fn default_number_of_workers() -> usize {
    8
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

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct RayNodeType {
    resources: Option<RayResources>,
    min_workers: usize,
    max_workers: usize,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
struct RayResources {
    cpu: usize,
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
                    resources: Some(RayResources { cpu: 0 }),
                    min_workers: daft_config.setup.number_of_workers,
                    max_workers: daft_config.setup.number_of_workers,
                },
            ),
            (
                "ray.worker.default".into(),
                RayNodeType {
                    resources: None,
                    min_workers: daft_config.setup.number_of_workers,
                    max_workers: daft_config.setup.number_of_workers,
                },
            ),
        ]
        .into_iter()
        .collect(),
    }
}

fn main() -> anyhow::Result<()> {
    let command = Command::parse();
    match command.sub_command {
        SubCommand::Init(Init { path }) => {
            if path.exists() {
                bail!("The path {path:?} already exists; the path given must point to a new location on your filesystem");
            };
            let contents = include_str!("default.toml");
            let contents = contents.replace("<VERSION>", env!("CARGO_PKG_VERSION"));
            fs::write(path, contents)?;
        }
        SubCommand::Up(Up { config }) => {
            let contents = fs::read_to_string(&config.path)?;
            let daft_config = dbg!(toml::from_str::<DaftConfig>(&contents)?);
            let ray_config = dbg!(convert(daft_config));
            let contents = serde_yaml::to_string(&ray_config)?;
            let temp_dir = tempdir::TempDir::new("daft-launcher")?;
            let mut path = temp_dir.path().to_owned();
            path.push("ray.yaml");
            fs::write(path, contents)?;
        }
    }

    Ok(())
}
