macro_rules! not_available_for_byoc {
    ($command:literal) => {
        anyhow::bail!(concat!(
            "The command `",
            $command,
            "` is only available for provisioned configurations"
        ))
    };
}

mod ssh;

use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    net::Ipv4Addr,
    path::{Path, PathBuf},
    process::Stdio,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

#[cfg(not(test))]
use anyhow::bail;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_ec2::{types::InstanceStateName, Client};
use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{
    modifiers, presets, Attribute, Cell, CellAlignment, Color, ContentArrangement, Table,
};
use serde::{Deserialize, Serialize};
use tempdir::TempDir;
use tokio::{
    fs,
    process::{Child, Command},
};
use versions::{Requirement, Versioning};

type StrRef = Arc<str>;
type PathRef = Arc<Path>;

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
#[command(name = env!("CARGO_PKG_NAME"), version = env!("CARGO_PKG_VERSION"), about = env!("CARGO_PKG_DESCRIPTION"))]
struct DaftLauncher {
    #[command(subcommand)]
    sub_command: SubCommand,
}

#[derive(Debug, Subcommand, Clone, PartialEq, Eq)]
enum SubCommand {
    /// Manage Daft-provisioned clusters (AWS)
    #[command(subcommand)]
    Provisioned(ProvisionedCommand),

    /// Manage existing clusters (Kubernetes)
    #[command(subcommand)]
    Byoc(ByocCommand),

    /// Manage jobs across all cluster types
    #[command(subcommand)]
    Job(JobCommand),

    /// Manage configurations
    #[command(subcommand)]
    Config(ConfigCommand),
}

#[derive(Debug, Subcommand, Clone, PartialEq, Eq)]
enum ProvisionedCommand {
    /// Create a new cluster
    Up(ConfigPath),

    /// Stop a running cluster
    Down(ConfigPath),

    /// Terminate a cluster
    Kill(ConfigPath),

    /// List all clusters
    List(List),

    /// Connect to cluster dashboard
    Connect(Connect),

    /// SSH into cluster head node
    Ssh(ConfigPath),
}

#[derive(Debug, Subcommand, Clone, PartialEq, Eq)]
enum ByocCommand {
    /// Verify connection to existing cluster
    Verify(ConfigPath),

    /// Show cluster information
    Info(ConfigPath),
}

#[derive(Debug, Subcommand, Clone, PartialEq, Eq)]
enum JobCommand {
    /// Submit a job to the cluster
    Submit(Submit),

    /// Execute SQL queries
    Sql(Sql),

    /// Check job status
    Status(ConfigPath),

    /// View job logs
    Logs(ConfigPath),
}

#[derive(Debug, Subcommand, Clone, PartialEq, Eq)]
enum ConfigCommand {
    /// Initialize a new configuration
    Init(Init),

    /// Validate configuration
    Check(ConfigPath),

    /// Export configuration to Ray format
    Export(ConfigPath),
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Init {
    /// The path at which to create the config file.
    #[arg(default_value = ".daft.toml")]
    path: PathBuf,

    /// The provider to use - either 'provisioned' (default) to auto-generate a cluster or 'byoc' for existing Kubernetes clusters
    #[arg(long, default_value_t = DaftProvider::Provisioned)]
    provider: DaftProvider,
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct List {
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
    #[serde(rename = "job", deserialize_with = "parse_jobs")]
    jobs: HashMap<StrRef, DaftJob>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct DaftSetup {
    name: StrRef,
    #[serde(deserialize_with = "parse_requirement")]
    version: Requirement,
    #[serde(flatten)]
    provider_config: ProviderConfig,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
enum ProviderConfig {
    Provisioned(AwsConfig),
    Byoc(K8sConfig),
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct AwsConfig {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    iam_instance_profile_name: Option<StrRef>,
    #[serde(default)]
    dependencies: Vec<StrRef>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct K8sConfig {
    namespace: Option<StrRef>,
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

fn parse_requirement<'de, D>(deserializer: D) -> Result<Requirement, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: StrRef = Deserialize::deserialize(deserializer)?;
    let requirement = raw
        .parse::<Requirement>()
        .map_err(serde::de::Error::custom)?;
    let current_version = env!("CARGO_PKG_VERSION")
        .parse::<Versioning>()
        .expect("CARGO_PKG_VERSION must exist");
    if requirement.matches(&current_version) {
        Ok(requirement)
    } else {
        Err(serde::de::Error::custom(format!("You're running daft-launcher version {current_version}, but your configuration file requires version {requirement}")))
    }
}

#[derive(Debug, ValueEnum, Clone, PartialEq, Eq)]
enum DaftProvider {
    Provisioned,
    Byoc,
}

impl ToString for DaftProvider {
    fn to_string(&self) -> String {
        match self {
            DaftProvider::Provisioned => "provisioned",
            DaftProvider::Byoc => "byoc",
        }
        .to_string()
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
    iam_instance_profile: Option<IamInstanceProfile>,
}

#[derive(Default, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
struct IamInstanceProfile {
    name: StrRef,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
struct RayResources {
    cpu: usize,
}

async fn read_daft_config(daft_config_path: impl AsRef<Path>) -> anyhow::Result<DaftConfig> {
    let daft_config_path = daft_config_path.as_ref();
    let contents = fs::read_to_string(daft_config_path)
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

    Ok(daft_config)
}

fn convert(
    daft_config: &DaftConfig,
    teardown_behaviour: Option<TeardownBehaviour>,
) -> anyhow::Result<RayConfig> {
    let ProviderConfig::Provisioned(aws_config) = &daft_config.setup.provider_config else {
        unreachable!("Can only convert to a ray config-file for provisioned configurations; this should be statically determined");
    };

    let key_name = aws_config
        .ssh_private_key
        .clone()
        .file_stem()
        .ok_or_else(|| {
            anyhow::anyhow!(r#"Private key doesn't have a name of the format "name.ext""#)
        })?
        .to_str()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "The file {:?} does not have a valid UTF-8 name",
                aws_config.ssh_private_key
            )
        })?
        .into();

    let node_config = RayNodeConfig {
        key_name,
        instance_type: aws_config.instance_type.clone(),
        image_id: aws_config.image_id.clone(),
        iam_instance_profile: aws_config
            .iam_instance_profile_name
            .clone()
            .map(|name| IamInstanceProfile { name }),
    };

    Ok(RayConfig {
        cluster_name: daft_config.setup.name.clone(),
        max_workers: aws_config.number_of_workers,
        provider: RayProvider {
            r#type: "aws".into(),
            region: aws_config.region.clone(),
            cache_stopped_nodes: teardown_behaviour.map(TeardownBehaviour::to_cache_stopped_nodes),
        },
        auth: RayAuth {
            ssh_user: aws_config.ssh_user.clone(),
            ssh_private_key: aws_config.ssh_private_key.clone(),
        },
        available_node_types: vec![
            (
                "ray.head.default".into(),
                RayNodeType {
                    max_workers: aws_config.number_of_workers,
                    node_config: node_config.clone(),
                    resources: Some(RayResources { cpu: 0 }),
                },
            ),
            (
                "ray.worker.default".into(),
                RayNodeType {
                    max_workers: aws_config.number_of_workers,
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
            if !aws_config.dependencies.is_empty() {
                let deps = aws_config
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

async fn write_ray_config(ray_config: &RayConfig, dest: impl AsRef<Path>) -> anyhow::Result<()> {
    let ray_config = serde_yaml::to_string(ray_config)?;
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
    Down,
    Kill,
}

impl TeardownBehaviour {
    fn to_cache_stopped_nodes(self) -> bool {
        match self {
            Self::Down => true,
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
enum NodeType {
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

fn print_instances(instances: &[AwsInstance], head: bool, running: bool) {
    let mut table = Table::default();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
        .apply_modifier(modifiers::UTF8_SOLID_INNER_BORDERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(["Name", "Instance ID", "Status", "IPv4"].map(|header| {
            Cell::new(header)
                .set_alignment(CellAlignment::Center)
                .add_attribute(Attribute::Bold)
        }));
    for instance in instances.iter().filter(|instance| {
        if head && instance.node_type != NodeType::Head {
            return false;
        } else if running && instance.state != Some(InstanceStateName::Running) {
            return false;
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
            Cell::new(&*instance.instance_id),
            status,
            Cell::new(ipv4),
        ]);
    }
    println!("{table}");
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

async fn establish_kubernetes_port_forward(namespace: Option<&str>) -> anyhow::Result<Child> {
    let namespace = namespace.unwrap_or("default");
    let output = Command::new("kubectl")
        .arg("get")
        .arg("svc")
        .arg("-n")
        .arg(namespace)
        .arg("-l")
        .arg("ray.io/node-type=head")
        .arg("--no-headers")
        .arg("-o")
        .arg("custom-columns=:metadata.name")
        .kill_on_drop(true)
        .output()
        .await?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to get Ray head node services with kubectl in namespace {}",
            namespace
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        anyhow::bail!("Ray head node service not found in namespace {}", namespace);
    }

    let head_node_service_name = stdout
        .lines()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to get the head node service name"))?;
    println!(
        "Found Ray head node service: {} in namespace {}",
        head_node_service_name, namespace
    );

    // Start port-forward with stderr piped so we can monitor the process
    let mut port_forward = Command::new("kubectl")
        .arg("port-forward")
        .arg("-n")
        .arg(namespace)
        .arg(format!("svc/{}", head_node_service_name))
        .arg("8265:8265")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped()) // Capture stdout too
        .kill_on_drop(true)
        .spawn()?;

    // Give the port-forward a moment to start and check for immediate failures
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check if process is still running
    match port_forward.try_wait()? {
        Some(status) => {
            anyhow::bail!(
                "Port-forward process exited immediately with status: {}",
                status
            );
        }
        None => {
            println!("Port-forwarding started successfully");
            Ok(port_forward)
        }
    }
}

async fn submit(
    working_dir: impl AsRef<Path>,
    command_segments: impl AsRef<[&str]>,
) -> anyhow::Result<()> {
    // Submit the job
    let exit_status = Command::new("ray")
        .env("PYTHONUNBUFFERED", "1")
        .args(["job", "submit", "--address", "http://localhost:8265"])
        .arg("--working-dir")
        .arg(working_dir.as_ref())
        .arg("--")
        .args(command_segments.as_ref())
        .spawn()?
        .wait()
        .await?;

    if exit_status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to submit job to the ray cluster"))
    }
}

async fn submit_k8s(
    working_dir: impl AsRef<Path>,
    command_segments: impl AsRef<[&str]>,
    namespace: Option<&str>,
) -> anyhow::Result<()> {
    // Start port forwarding - it will be automatically killed when _port_forward is dropped
    let _port_forward = establish_kubernetes_port_forward(namespace).await?;

    // Give the port-forward a moment to fully establish
    tokio::time::sleep(Duration::from_secs(1)).await;

    submit(working_dir, command_segments).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    DaftLauncher::parse().run().await
}

impl DaftLauncher {
    async fn run(&self) -> anyhow::Result<()> {
        match &self.sub_command {
            SubCommand::Config(config_cmd) => config_cmd.run().await,
            SubCommand::Job(job_cmd) => job_cmd.run().await,
            SubCommand::Provisioned(provisioned_cmd) => provisioned_cmd.run().await,
            SubCommand::Byoc(byoc_cmd) => byoc_cmd.run().await,
        }
    }
}

impl ConfigCommand {
    async fn run(&self) -> anyhow::Result<()> {
        match self {
            ConfigCommand::Init(Init { path, provider }) => {
                #[cfg(not(test))]
                if path.exists() {
                    bail!("The path {path:?} already exists; the path given must point to a new location on your filesystem");
                }
                let contents = match provider {
                    DaftProvider::Byoc => include_str!("template_byoc.toml"),
                    DaftProvider::Provisioned => include_str!("template_provisioned.toml"),
                }
                .replace("<VERSION>", concat!("=", env!("CARGO_PKG_VERSION")));
                fs::write(path, contents).await?;
            }
            ConfigCommand::Check(ConfigPath { config }) => {
                let _ = read_daft_config(config).await?;
            }
            ConfigCommand::Export(ConfigPath { config }) => {
                let daft_config = read_daft_config(config).await?;
                let ray_config = convert(&daft_config, None)?;
                let ray_config_str = serde_yaml::to_string(&ray_config)?;
                println!("{ray_config_str}");
            }
        }
        Ok(())
    }
}

impl JobCommand {
    async fn run(&self) -> anyhow::Result<()> {
        match self {
            JobCommand::Submit(Submit {
                config_path,
                job_name,
            }) => {
                let daft_config = read_daft_config(&config_path.config).await?;
                let daft_job = daft_config.jobs.get(job_name).ok_or_else(|| {
                    anyhow::anyhow!("A job with the name {job_name} was not found")
                })?;

                let working_dir = daft_job.working_dir.as_ref();
                let command_segments = daft_job.command.as_ref().split(' ').collect::<Vec<_>>();

                match &daft_config.setup.provider_config {
                    ProviderConfig::Provisioned(aws_config) => {
                        assert_is_logged_in_with_aws().await?;

                        let ray_config = convert(&daft_config, None)?;
                        let (_temp_dir, ray_path) = create_temp_ray_file()?;
                        write_ray_config(&ray_config, &ray_path).await?;

                        let _child = ssh::ssh_portforward(ray_path, aws_config, None).await?;
                        submit(working_dir, command_segments).await?;
                    }
                    ProviderConfig::Byoc(k8s_config) => {
                        submit_k8s(
                            working_dir,
                            command_segments,
                            k8s_config.namespace.as_deref(),
                        )
                        .await?;
                    }
                }
            }
            JobCommand::Sql(Sql { sql, config_path }) => {
                let daft_config = read_daft_config(&config_path.config).await?;
                let (temp_sql_dir, sql_path) = create_temp_file("sql.py")?;
                fs::write(sql_path, include_str!("sql.py")).await?;

                let working_dir = temp_sql_dir.path();
                let command_segments = vec!["python", "sql.py", sql.as_ref()];

                match &daft_config.setup.provider_config {
                    ProviderConfig::Provisioned(aws_config) => {
                        assert_is_logged_in_with_aws().await?;

                        let ray_config = convert(&daft_config, None)?;
                        let (_temp_dir, ray_path) = create_temp_ray_file()?;
                        write_ray_config(&ray_config, &ray_path).await?;

                        let _child = ssh::ssh_portforward(ray_path, aws_config, None).await?;
                        submit(working_dir, command_segments).await?;
                    }
                    ProviderConfig::Byoc(k8s_config) => {
                        submit_k8s(
                            working_dir,
                            command_segments,
                            k8s_config.namespace.as_deref(),
                        )
                        .await?;
                    }
                }
            }
            JobCommand::Status(..) => todo!(),
            JobCommand::Logs(..) => todo!(),
        }
        Ok(())
    }
}

impl ProvisionedCommand {
    async fn run(&self) -> anyhow::Result<()> {
        match self {
            ProvisionedCommand::Up(ConfigPath { config }) => {
                let daft_config = read_daft_config(config).await?;
                match daft_config.setup.provider_config {
                    ProviderConfig::Provisioned(..) => {
                        assert_is_logged_in_with_aws().await?;

                        let ray_config = convert(&daft_config, None)?;
                        let (_temp_dir, ray_path) = create_temp_ray_file()?;
                        write_ray_config(&ray_config, &ray_path).await?;
                        run_ray_up_or_down_command(SpinDirection::Up, ray_path).await?;
                    }
                    ProviderConfig::Byoc(..) => not_available_for_byoc!("up"),
                }
            }
            ProvisionedCommand::Down(ConfigPath { config }) => {
                let daft_config = read_daft_config(config).await?;
                match daft_config.setup.provider_config {
                    ProviderConfig::Provisioned(..) => {
                        assert_is_logged_in_with_aws().await?;

                        let ray_config = convert(&daft_config, Some(TeardownBehaviour::Down))?;
                        let (_temp_dir, ray_path) = create_temp_ray_file()?;
                        write_ray_config(&ray_config, &ray_path).await?;
                        run_ray_up_or_down_command(SpinDirection::Down, ray_path).await?;
                    }
                    ProviderConfig::Byoc(..) => not_available_for_byoc!("down"),
                }
            }
            ProvisionedCommand::Kill(ConfigPath { config }) => {
                let daft_config = read_daft_config(config).await?;
                match daft_config.setup.provider_config {
                    ProviderConfig::Provisioned(..) => {
                        assert_is_logged_in_with_aws().await?;

                        let ray_config = convert(&daft_config, Some(TeardownBehaviour::Kill))?;
                        let (_temp_dir, ray_path) = create_temp_ray_file()?;
                        write_ray_config(&ray_config, &ray_path).await?;
                        run_ray_up_or_down_command(SpinDirection::Down, ray_path).await?;
                    }
                    ProviderConfig::Byoc(..) => not_available_for_byoc!("kill"),
                }
            }
            ProvisionedCommand::List(List {
                config_path,
                region,
                head,
                running,
            }) => {
                let daft_config = read_daft_config(&config_path.config).await?;
                match &daft_config.setup.provider_config {
                    ProviderConfig::Provisioned(aws_config) => {
                        assert_is_logged_in_with_aws().await?;

                        let region = region.as_ref().unwrap_or_else(|| &aws_config.region);
                        let instances = get_ray_clusters_from_aws(region.clone()).await?;
                        print_instances(&instances, *head, *running);
                    }
                    ProviderConfig::Byoc(..) => not_available_for_byoc!("list"),
                }
            }
            &ProvisionedCommand::Connect(Connect {
                port,
                ref config_path,
            }) => {
                let daft_config = read_daft_config(&config_path.config).await?;
                match &daft_config.setup.provider_config {
                    ProviderConfig::Provisioned(aws_config) => {
                        assert_is_logged_in_with_aws().await?;

                        let ray_config = convert(&daft_config, None)?;
                        let (_temp_dir, ray_path) = create_temp_ray_file()?;
                        write_ray_config(&ray_config, &ray_path).await?;
                        let _ = ssh::ssh_portforward(ray_path, aws_config, Some(port))
                            .await?
                            .wait_with_output()
                            .await?;
                    }
                    ProviderConfig::Byoc(..) => not_available_for_byoc!("connect"),
                }
            }
            ProvisionedCommand::Ssh(ConfigPath { config }) => {
                let daft_config = read_daft_config(config).await?;
                match &daft_config.setup.provider_config {
                    ProviderConfig::Provisioned(aws_config) => {
                        assert_is_logged_in_with_aws().await?;

                        let ray_config = convert(&daft_config, None)?;
                        let (_temp_dir, ray_path) = create_temp_ray_file()?;
                        write_ray_config(&ray_config, &ray_path).await?;
                        ssh::ssh(ray_path, aws_config).await?;
                    }
                    ProviderConfig::Byoc(..) => not_available_for_byoc!("ssh"),
                }
            }
        }
        Ok(())
    }
}

impl ByocCommand {
    async fn run(&self) -> anyhow::Result<()> {
        match self {
            ByocCommand::Verify(..) => todo!(),
            ByocCommand::Info(..) => todo!(),
        }
    }
}
