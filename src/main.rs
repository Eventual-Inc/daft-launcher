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
use clap::{Parser, Subcommand};
use comfy_table::{
    modifiers, presets, Attribute, Cell, CellAlignment, Color, ContentArrangement, Table,
};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use tempdir::TempDir;
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    time::timeout,
};

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
    #[arg(long)]
    region: Option<StrRef>,

    /// Only list the head nodes.
    #[arg(long)]
    head: bool,

    /// Only list the running instances.
    #[arg(long)]
    running: bool,
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
    run: DaftRun,
    #[serde(rename = "job", deserialize_with = "parse_jobs")]
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

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
enum DaftProvider {
    Aws,
}

#[derive(Default, Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct DaftRun {
    #[serde(default)]
    pre_setup_commands: Vec<StrRef>,
    #[serde(default)]
    post_setup_commands: Vec<StrRef>,
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
        let iam_instance_profile = daft_config
            .setup
            .iam_instance_profile_name
            .clone()
            .map(|name| IamInstanceProfile { name });
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

async fn establish_ssh_portforward(
    ray_path: impl AsRef<Path>,
    daft_config: &DaftConfig,
    port: Option<u16>,
) -> anyhow::Result<Child> {
    let user = daft_config.setup.ssh_user.as_ref();
    let addr = get_head_node_ip(ray_path).await?;
    let port = port.unwrap_or(8265);
    let mut child = Command::new("ssh")
        .arg("-N")
        .arg("-i")
        .arg(daft_config.setup.ssh_private_key.as_ref())
        .arg("-L")
        .arg(format!("{port}:localhost:8265"))
        .arg(format!("{user}@{addr}"))
        .arg("-v")
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

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
            let (_, ray_config) = read_and_convert(&config, Some(TeardownBehaviour::Stop)).await?;
            assert_is_logged_in_with_aws().await?;

            let (_temp_dir, ray_path) = create_temp_ray_file()?;
            write_ray_config(ray_config, &ray_path).await?;
            run_ray_up_or_down_command(SpinDirection::Up, ray_path).await?;
        }
        SubCommand::List(List {
            config_path,
            region,
            head,
            running,
        }) => {
            assert_is_logged_in_with_aws().await?;

            let region = get_region(region, &config_path.config).await?;
            let instances = get_ray_clusters_from_aws(region).await?;
            print_instances(&instances, head, running);
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
            let _child = establish_ssh_portforward(ray_path, &daft_config, None).await?;

            let exit_status = Command::new("ray")
                .env("PYTHONUNBUFFERED", "1")
                .args(["job", "submit", "--address", "http://localhost:8265"])
                .arg("--working-dir")
                .arg(daft_job.working_dir.as_ref())
                .arg("--")
                .args(daft_job.command.split(' '))
                .spawn()?
                .wait()
                .await?;
            if !exit_status.success() {
                anyhow::bail!("Failed to submit job to the ray cluster");
            };
        }
        SubCommand::Connect(Connect { port, config_path }) => {
            let (daft_config, ray_config) = read_and_convert(&config_path.config, None).await?;
            assert_is_logged_in_with_aws().await?;

            let (_temp_dir, ray_path) = create_temp_ray_file()?;
            write_ray_config(ray_config, &ray_path).await?;
            let _ = establish_ssh_portforward(ray_path, &daft_config, Some(port))
                .await?
                .wait_with_output()
                .await?;
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
