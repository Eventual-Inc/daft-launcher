use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    net::Ipv4Addr,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

#[cfg(not(test))]
use anyhow::bail;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_ec2::types::InstanceStateName;
use aws_sdk_ec2::Client;
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

    /// List all Ray clusters in your AWS account.
    ///
    /// This will *only* list clusters that have been spun up by Ray.
    List(List),

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
struct List {
    #[clap(flatten)]
    config_path: ConfigPath,

    /// The region which to list all the available clusters for.
    #[arg(default_value = "us-west-2")]
    region: StrRef,

    /// Only list the head nodes.
    #[arg(long)]
    head: bool,

    /// Only list the running instances.
    #[arg(long)]
    running: bool,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_stopped_nodes: Option<bool>,
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

async fn read_and_convert(
    daft_config_path: &Path,
    teardown_behaviour: Option<TeardownBehaviour>,
) -> anyhow::Result<(DaftConfig, RayConfig)> {
    fn convert(
        daft_config: &DaftConfig,
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
            cluster_name: daft_config.setup.name.clone(),
            max_workers: daft_config.setup.number_of_workers,
            provider: RayProvider {
                r#type: "aws".into(),
                region: "us-west-2".into(),
                cache_stopped_nodes: teardown_behaviour
                    .map(TeardownBehaviour::to_cache_stopped_nodes),
            },
            auth: RayAuth {
                ssh_user: daft_config.setup.ssh_user.clone(),
                ssh_private_key: daft_config.setup.ssh_private_key.clone(),
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
    let ray_config = convert(&daft_config, teardown_behaviour)?;

    Ok((daft_config, ray_config))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TeardownBehaviour {
    Stop,
    Kill,
}

impl TeardownBehaviour {
    fn to_cache_stopped_nodes(self) -> bool {
        match self {
            Self::Stop => true,
            Self::Kill => false,
        }
    }
}

fn create_temp_ray_file() -> anyhow::Result<(TempDir, PathRef)> {
    let temp_dir = TempDir::new("daft-launcher")?;
    let mut ray_path = temp_dir.path().to_owned();
    ray_path.push("ray.yaml");
    let ray_path = Arc::from(ray_path);
    Ok((temp_dir, ray_path))
}

async fn run_ray_command(
    spin_direction: SpinDirection,
    ray_path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let _ = Command::new("ray")
        .arg(spin_direction.as_str())
        .arg(ray_path.as_ref())
        .arg("-y")
        .spawn()?
        .wait_with_output()
        .await?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AwsInstance {
    instance_id: StrRef,
    regular_name: StrRef,
    ray_name: StrRef,
    key_pair_name: Option<StrRef>,
    public_ipv4_address: Option<Ipv4Addr>,
    state: Option<InstanceStateName>,
    node_type: NodeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Head,
    Worker,
}

impl FromStr for NodeType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "head" => Ok(NodeType::Head),
            "worker" => Ok(NodeType::Worker),
            _ => anyhow::bail!("Unrecognized node type: {}", s),
        }
    }
}

async fn get_ray_clusters_from_aws(region: StrRef) -> anyhow::Result<Vec<AwsInstance>> {
    let region = Region::new(region.to_string());
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .load()
        .await;
    let client = Client::new(&sdk_config);
    let instances = client.describe_instances().send().await?;
    let reservations = instances.reservations.unwrap_or_default();
    let instance_states = reservations
        .iter()
        .filter_map(|reservation| reservation.instances.as_ref())
        .flatten()
        .filter_map(|instance| {
            instance.tags.as_ref().map(|tags| {
                (
                    instance,
                    tags.iter().filter_map(|tag| tag.key().zip(tag.value())),
                )
            })
        })
        .filter_map(|(instance, tags)| {
            let mut ray_name = None;
            let mut regular_name = None;
            let mut node_type = None;
            for (key, value) in tags {
                if key == "Name" {
                    ray_name = Some(value.into());
                } else if key == "ray-cluster-name" {
                    regular_name = Some(value.into());
                } else if key == "ray-node-type" {
                    node_type = value.parse().ok();
                }
            }
            let ray_name = ray_name?;
            let regular_name = regular_name?;
            let instance_id = instance.instance_id.as_deref()?.into();
            let node_type = node_type?;
            Some(AwsInstance {
                instance_id,
                regular_name,
                ray_name,
                key_pair_name: instance.key_name().map(Into::into),
                public_ipv4_address: instance
                    .public_ip_address()
                    .and_then(|ip_addr| ip_addr.parse().ok()),
                state: instance
                    .state()
                    .and_then(|instance_state| instance_state.name())
                    .cloned(),
                node_type,
            })
        })
        .collect();
    Ok(instance_states)
}

async fn assert_is_logged_in_with_aws() -> anyhow::Result<()> {
    let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::meta::region::RegionProviderChain::default_provider())
        .load()
        .await;
    let client = aws_sdk_sts::Client::new(&sdk_config);
    if client.get_caller_identity().send().await.is_ok() {
        Ok(())
    } else {
        anyhow::bail!("You are not logged in with the AWS cli tool; please authenticate with it first before re-running")
    }
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
        SubCommand::Check(ConfigPath { config }) => {
            let _ = read_and_convert(&config, None).await?;
        }
        SubCommand::Export(ConfigPath { config }) => {
            let (_, ray_config) = read_and_convert(&config, Some(TeardownBehaviour::Stop)).await?;
            let ray_config_str = serde_yaml::to_string(&ray_config)?;
            println!("{ray_config_str}");
        }
        SubCommand::Up(ConfigPath { config }) => {
            let (_, ray_path) = create_temp_ray_file()?;
            let (_, ray_config) = read_and_convert(&config, Some(TeardownBehaviour::Stop)).await?;
            write_ray_config(ray_config, &ray_path).await?;
            assert_is_logged_in_with_aws().await?;
            run_ray_command(SpinDirection::Up, ray_path).await?;
        }
        SubCommand::List(List {
            config_path,
            region,
            head,
            running,
        }) => {
            let region = if config_path.config.exists() {
                let (daft_config, _) = read_and_convert(&config_path.config, None).await?;
                daft_config.setup.region
            } else {
                region
            };
            assert_is_logged_in_with_aws().await?;
            let instances = get_ray_clusters_from_aws(region).await?;
            for instance in instances.iter().filter(|instance| {
                if running && instance.state != Some(InstanceStateName::Running) {
                    return false;
                } else if head && instance.node_type != NodeType::Head {
                    return false;
                };

                true
            }) {
                println!("{instance:?}");
            }
        }
        SubCommand::Stop(ConfigPath { config }) => {
            let (_, ray_path) = create_temp_ray_file()?;
            let (_, ray_config) = read_and_convert(&config, Some(TeardownBehaviour::Stop)).await?;
            write_ray_config(ray_config, &ray_path).await?;
            assert_is_logged_in_with_aws().await?;
            run_ray_command(SpinDirection::Down, ray_path).await?;
        }
        SubCommand::Kill(ConfigPath { config }) => {
            let (_, ray_path) = create_temp_ray_file()?;
            let (_, ray_config) = read_and_convert(&config, Some(TeardownBehaviour::Kill)).await?;
            write_ray_config(ray_config, &ray_path).await?;
            assert_is_logged_in_with_aws().await?;
            run_ray_command(SpinDirection::Down, ray_path).await?;
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
