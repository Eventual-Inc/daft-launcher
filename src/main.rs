mod ssh;
#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    net::Ipv4Addr,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    thread::{sleep, spawn},
    time::Duration,
};

#[cfg(not(test))]
use anyhow::bail;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_ec2::{types::InstanceStateName, Client};
use clap::{Parser, Subcommand};
use comfy_table::{
    modifiers, presets, Attribute, Cell, CellAlignment, Color, ContentArrangement, Table,
};
use regex::Regex;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use tempdir::TempDir;
use tokio::{fs, process::Command};

type StrRef = Arc<str>;
type PathRef = Arc<Path>;

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
#[command(name = env!("CARGO_PKG_NAME"), version = env!("CARGO_PKG_VERSION"), about = env!("CARGO_PKG_DESCRIPTION"))]
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

    /// Export the daft-launcher configuration file to a Ray configuration file.
    Export(ConfigPath),

    /// Spin up a new cluster.
    Up(ConfigPath),

    /// List all Ray clusters in your AWS account.
    ///
    /// This will *only* list clusters that have been spun up by Ray.
    List(List),

    /// Submit a job to the Ray cluster.
    ///
    /// The configurations of the job should be placed inside of your
    /// daft-launcher configuration file.
    Submit(Submit),

    /// Establish an ssh port-forward connection from your local machine to the
    /// Ray cluster.
    Connect(Connect),

    /// SSH into the head of the remote Ray cluster.
    Ssh(ConfigPath),

    /// Submit a SQL query string to the Ray cluster.
    ///
    /// This is executed using Daft's SQL API support.
    Sql(Sql),

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
    /// A regex to filter for the Ray clusters which match the given name.
    regex: Option<StrRef>,

    /// The region which to list all the available clusters for.
    #[arg(long)]
    region: Option<StrRef>,

    /// Only list the head nodes.
    #[arg(long)]
    head: bool,

    /// Only list the running instances.
    #[arg(long)]
    running: bool,

    #[clap(flatten)]
    config_path: ConfigPath,
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Submit {
    /// The name of the job to run.
    job_name: StrRef,

    #[clap(flatten)]
    config_path: ConfigPath,
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Connect {
    /// The local port to connect to the remote Ray cluster.
    #[arg(long, default_value = "8265")]
    port: u16,

    /// Don't open the dashboard automatically.
    #[arg(long)]
    no_dashboard: bool,

    #[clap(flatten)]
    config_path: ConfigPath,
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Sql {
    /// The SQL string to submit to the remote Ray cluster.
    sql: StrRef,

    #[clap(flatten)]
    config_path: ConfigPath,
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct ConfigPath {
    /// Path to configuration file.
    #[arg(default_value = ".daft.toml")]
    config: PathBuf,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct DaftConfig {
    setup: DaftSetup,
    #[serde(default)]
    run: Vec<StrRef>,
    #[serde(default, rename = "job", deserialize_with = "parse_jobs")]
    jobs: HashMap<StrRef, DaftJob>,
}

fn parse_jobs<'de, D>(deserializer: D) -> Result<HashMap<StrRef, DaftJob>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
    #[serde(rename_all = "kebab-case")]
    struct Job {
        name: StrRef,
        command: StrRef,
        working_dir: PathRef,
    }

    let jobs: Vec<Job> = Deserialize::deserialize(deserializer)?;
    let jobs = jobs
        .into_iter()
        .map(|job| {
            let working_dir = expand_and_check_path(job.working_dir)?;
            Ok((
                job.name,
                DaftJob {
                    command: job.command,
                    working_dir,
                },
            ))
        })
        .collect::<anyhow::Result<_>>()
        .map_err(serde::de::Error::custom)?;
    Ok(jobs)
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct DaftSetup {
    name: StrRef,
    #[serde(deserialize_with = "parse_version_req")]
    version: VersionReq,
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
    iam_instance_profile_name: Option<StrRef>,
    #[serde(default)]
    dependencies: Vec<StrRef>,
}

fn parse_ssh_private_key<'de, D>(deserializer: D) -> Result<PathRef, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let path: PathRef = Deserialize::deserialize(deserializer)?;
    let path = expand_and_check_path(path).map_err(serde::de::Error::custom)?;
    Ok(path)
}

fn expand_and_check_path(path: PathRef) -> anyhow::Result<PathRef> {
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
            anyhow::bail!("The path, {path:?}, does not exist")
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
    let current_version = env!("CARGO_PKG_VERSION")
        .parse::<Version>()
        .expect("CARGO_PKG_VERSION must exist");
    if version_req.matches(&current_version) {
        Ok(version_req)
    } else {
        Err(serde::de::Error::custom(format!("You're running daft-launcher version {current_version}, but your configuration file requires version {version_req}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DaftJob {
    command: StrRef,
    working_dir: PathRef,
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
    iam_instance_profile: Option<RayIamInstanceProfile>,
}

#[derive(Default, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
struct RayIamInstanceProfile {
    name: StrRef,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
struct RayResources {
    cpu: usize,
}

fn default_setup_commands() -> Vec<StrRef> {
    vec![
        "curl -LsSf https://astral.sh/uv/install.sh | sh".into(),
        "uv python install 3.12".into(),
        "uv python pin 3.12".into(),
        "uv venv".into(),
        "echo 'source $HOME/.venv/bin/activate' >> ~/.bashrc".into(),
        "source ~/.bashrc".into(),
        "uv pip install boto3 pip ray[default] getdaft py-spy deltalake".into(),
    ]
}

fn convert(
    daft_config: &DaftConfig,
    teardown_behaviour: Option<TeardownBehaviour>,
) -> anyhow::Result<RayConfig> {
    let key_name = daft_config
        .setup
        .ssh_private_key
        .clone()
        .file_stem()
        .ok_or_else(|| {
            anyhow::anyhow!(r#"Private key doesn't have a name of the format "name.ext""#)
        })?
        .to_str()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "The file {:?} does not a valid UTF-8 name",
                daft_config.setup.ssh_private_key,
            )
        })?
        .into();
    let iam_instance_profile = daft_config
        .setup
        .iam_instance_profile_name
        .clone()
        .map(|name| RayIamInstanceProfile { name });
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
            region: daft_config.setup.region.clone(),
            cache_stopped_nodes: teardown_behaviour.map(TeardownBehaviour::to_cache_stopped_nodes),
        },
        auth: RayAuth {
            ssh_user: daft_config.setup.ssh_user.clone(),
            ssh_private_key: daft_config.setup.ssh_private_key.clone(),
        },
        available_node_types: vec![
            (
                "ray.head.default".into(),
                RayNodeType {
                    max_workers: 0,
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
            let mut commands = default_setup_commands();
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

async fn read_and_convert(
    daft_config_path: &Path,
    teardown_behaviour: Option<TeardownBehaviour>,
) -> anyhow::Result<(DaftConfig, RayConfig)> {
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

fn create_temp_file(name: &str) -> anyhow::Result<(TempDir, PathRef)> {
    let temp_dir = TempDir::new("daft-launcher")?;
    let mut temp_path = temp_dir.path().to_owned();
    temp_path.push(name);
    let temp_path = Arc::from(temp_path);
    Ok((temp_dir, temp_path))
}

fn create_temp_ray_file() -> anyhow::Result<(TempDir, PathRef)> {
    create_temp_file("ray.yaml")
}

async fn run_ray_up_or_down_command(
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

impl NodeType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Worker => "worker",
        }
    }
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

fn format_table(
    instances: &[AwsInstance],
    regex: Option<StrRef>,
    head: bool,
    running: bool,
) -> anyhow::Result<Table> {
    let mut table = Table::default();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
        .apply_modifier(modifiers::UTF8_SOLID_INNER_BORDERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(
            ["Name", "Instance ID", "Node Type", "Status", "IPv4"].map(|header| {
                Cell::new(header)
                    .set_alignment(CellAlignment::Center)
                    .add_attribute(Attribute::Bold)
            }),
        );
    let regex = regex.as_deref().map(Regex::new).transpose()?;
    for instance in instances.iter().filter(|instance| {
        if head && instance.node_type != NodeType::Head {
            return false;
        } else if running && instance.state != Some(InstanceStateName::Running) {
            return false;
        };
        if let Some(regex) = regex.as_ref() {
            if !regex.is_match(&instance.regular_name) {
                return false;
            };
        };
        true
    }) {
        let status = instance.state.as_ref().map_or_else(
            || Cell::new("n/a").add_attribute(Attribute::Dim),
            |status| {
                let cell = Cell::new(status);
                match status {
                    InstanceStateName::Running => cell.fg(Color::Green),
                    InstanceStateName::Pending => cell.fg(Color::Yellow),
                    InstanceStateName::ShuttingDown | InstanceStateName::Stopping => {
                        cell.fg(Color::DarkYellow)
                    }
                    InstanceStateName::Stopped | InstanceStateName::Terminated => {
                        cell.fg(Color::Red)
                    }
                    _ => cell,
                }
            },
        );
        let ipv4 = instance
            .public_ipv4_address
            .as_ref()
            .map_or("n/a".into(), ToString::to_string);
        table.add_row(vec![
            Cell::new(instance.regular_name.to_string()).fg(Color::Cyan),
            Cell::new(instance.instance_id.as_ref()),
            Cell::new(instance.node_type.as_str()),
            status,
            Cell::new(ipv4),
        ]);
    }
    Ok(table)
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

async fn get_region(region: Option<StrRef>, config: impl AsRef<Path>) -> anyhow::Result<StrRef> {
    let config = config.as_ref();
    Ok(if let Some(region) = region {
        region
    } else if config.exists() {
        let (daft_config, _) = read_and_convert(&config, None).await?;
        daft_config.setup.region
    } else {
        "us-west-2".into()
    })
}

async fn submit(working_dir: &Path, command_segments: impl AsRef<[&str]>) -> anyhow::Result<()> {
    let command_segments = command_segments.as_ref();

    let exit_status = Command::new("ray")
        .env("PYTHONUNBUFFERED", "1")
        .args(["job", "submit", "--address", "http://localhost:8265"])
        .arg("--working-dir")
        .arg(working_dir)
        .arg("--")
        .args(command_segments)
        .spawn()?
        .wait()
        .await?;
    if exit_status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to submit job to the ray cluster"))
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
            let (_, ray_config) = read_and_convert(&config, None).await?;
            let ray_config_str = serde_yaml::to_string(&ray_config)?;
            println!("{ray_config_str}");
        }
        SubCommand::Up(ConfigPath { config }) => {
            let (_, ray_config) = read_and_convert(&config, None).await?;
            assert_is_logged_in_with_aws().await?;

            let (_temp_dir, ray_path) = create_temp_ray_file()?;
            write_ray_config(ray_config, &ray_path).await?;
            run_ray_up_or_down_command(SpinDirection::Up, ray_path).await?;
        }
        SubCommand::List(List {
            regex,
            config_path,
            region,
            head,
            running,
        }) => {
            assert_is_logged_in_with_aws().await?;

            let region = get_region(region, &config_path.config).await?;
            let instances = get_ray_clusters_from_aws(region).await?;
            let table = format_table(&instances, regex, head, running)?;
            println!("{table}");
        }
        SubCommand::Submit(Submit {
            config_path,
            job_name,
        }) => {
            let (daft_config, ray_config) = read_and_convert(&config_path.config, None).await?;
            assert_is_logged_in_with_aws().await?;
            let daft_job = daft_config
                .jobs
                .get(&job_name)
                .ok_or_else(|| anyhow::anyhow!("A job with the name {job_name} was not found"))?;

            let (_temp_dir, ray_path) = create_temp_ray_file()?;
            write_ray_config(ray_config, &ray_path).await?;
            let _child = ssh::ssh_portforward(ray_path, &daft_config, None).await?;
            submit(
                daft_job.working_dir.as_ref(),
                daft_job.command.as_ref().split(' ').collect::<Vec<_>>(),
            )
            .await?;
        }
        SubCommand::Connect(Connect {
            port,
            no_dashboard,
            config_path,
        }) => {
            let (daft_config, ray_config) = read_and_convert(&config_path.config, None).await?;
            assert_is_logged_in_with_aws().await?;

            let (_temp_dir, ray_path) = create_temp_ray_file()?;
            write_ray_config(ray_config, &ray_path).await?;
            let open_join_handle = if !no_dashboard {
                Some(spawn(|| {
                    sleep(Duration::from_millis(500));
                    open::that("http://localhost:8265")?;
                    Ok::<_, anyhow::Error>(())
                }))
            } else {
                None
            };

            let _ = ssh::ssh_portforward(ray_path, &daft_config, Some(port))
                .await?
                .wait_with_output()
                .await?;

            if let Some(open_join_handle) = open_join_handle {
                open_join_handle
                    .join()
                    .map_err(|_| anyhow::anyhow!("Failed to join browser-opening thread"))??;
            };
        }
        SubCommand::Ssh(ConfigPath { config }) => {
            let (daft_config, ray_config) = read_and_convert(&config, None).await?;
            assert_is_logged_in_with_aws().await?;

            let (_temp_dir, ray_path) = create_temp_ray_file()?;
            write_ray_config(ray_config, &ray_path).await?;
            ssh::ssh(ray_path, &daft_config).await?;
        }
        SubCommand::Sql(Sql { sql, config_path }) => {
            let (daft_config, ray_config) = read_and_convert(&config_path.config, None).await?;
            assert_is_logged_in_with_aws().await?;

            let (_temp_dir, ray_path) = create_temp_ray_file()?;
            write_ray_config(ray_config, &ray_path).await?;
            let _child = ssh::ssh_portforward(ray_path, &daft_config, None).await?;
            let (temp_sql_dir, sql_path) = create_temp_file("sql.py")?;
            fs::write(sql_path, include_str!("sql.py")).await?;
            submit(temp_sql_dir.path(), vec!["python", "sql.py", sql.as_ref()]).await?;
        }
        SubCommand::Stop(ConfigPath { config }) => {
            let (_, ray_config) = read_and_convert(&config, Some(TeardownBehaviour::Stop)).await?;
            assert_is_logged_in_with_aws().await?;

            let (_temp_dir, ray_path) = create_temp_ray_file()?;
            write_ray_config(ray_config, &ray_path).await?;
            run_ray_up_or_down_command(SpinDirection::Down, ray_path).await?;
        }
        SubCommand::Kill(ConfigPath { config }) => {
            let (_, ray_config) = read_and_convert(&config, Some(TeardownBehaviour::Kill)).await?;
            assert_is_logged_in_with_aws().await?;

            let (_temp_dir, ray_path) = create_temp_ray_file()?;
            write_ray_config(ray_config, &ray_path).await?;
            run_ray_up_or_down_command(SpinDirection::Down, ray_path).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run(DaftLauncher::parse()).await
}
