use std::path::PathBuf;

use aws_sdk_ec2::types::InstanceStateName;
use clap::Parser;
use comfy_table::{
    modifiers::{UTF8_ROUND_CORNERS, UTF8_SOLID_INNER_BORDERS},
    presets::UTF8_FULL,
    Attribute, Cell, CellAlignment, Color, ContentArrangement, Table,
};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use regex::Regex;
use tokio::io::AsyncWriteExt;

use crate::{
    aws::{
        assert_authenticated as assert_authenticated_with_aws, assert_non_clashing_cluster_name,
        list_instances, AwsInstance,
    },
    config::{
        defaults::{normal_image_id, normal_instance_type},
        processed,
        raw::{
            default_name, AwsCluster, AwsCustomType, AwsTemplateType, Cluster, Package, Provider,
            RawConfig,
        },
        read, Selectable,
    },
    ray::{run_ray, RaySubcommand},
    utils::{assert_file_status, create_new_file, Status},
    ArcStrRef, StrRef,
};

#[derive(Parser)]
#[command(version, about = env!("CARGO_PKG_DESCRIPTION"))]
pub enum Cli {
    Init(Init),
    Up(Up),
    Down(Down),
    List(List),
    Submit(Submit),
    Connect(Connect),
    Dashboard(Dashboard),
    Sql(Sql),
}

/// Initialize a configuration file.
///
/// If nothing is provided, a configuration file name `.daft.toml` will be
/// created in the current directory. These configuration files are the entry
/// point into the daft launcher utility tool. They contain all the necessary
/// information to spin up a cluster, submit jobs, and more.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Init {
    /// Name of the configuration file (can be specified as a path).
    #[arg(short, long, default_value = ".daft.toml")]
    pub name: PathBuf,
    /// Skip interactive mode and generate a default configuration file.
    #[arg(short, long, default_value = "false")]
    pub default: bool,
}

/// Spin up a new cluster.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Up {
    #[clap(flatten)]
    pub config: Config,
}

/// Spin down a new cluster.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Down {
    #[clap(flatten)]
    pub config: Config,
    /// Name of the cluster to spin down.
    #[arg(short, long)]
    pub name: Option<ArcStrRef>,
    /// The cloud provider which contains the cluster to spin down.
    #[arg(short, long)]
    pub provider: Option<ArcStrRef>,
    /// Region of the cluster to spin down.
    #[arg(short, long)]
    pub region: Option<ArcStrRef>,
}

/// List all ray-clusters in each cloud provider.
///
/// This will list all of the clusters, regardless of their state (i.e.,
/// running, stopped, etc.). You can filter the results by appending any of the
/// supported flags.
///
/// Please note that this command will not list any *non-ray-clusters*! We only
/// list instances if and only if they have a "ray-cluster-name" tag on them. If
/// not, we assume that they have *not* been instantiated by ray, and
/// as a result, we don't list them.
#[derive(Debug, Parser, Clone)]
pub struct List {
    /// Only list the clusters that are running.
    #[arg(short, long)]
    pub running: bool,
    /// Only list the clusters that match this regex.
    #[arg(short, long)]
    pub name: Option<Regex>,
}

// fn parse_regex(s: &str) -> anyhow::Result<Regex> {
//     todo!()
//     // Regex::from_str(s).map_err(|e| e.to_string())
// }

/// Submit a job to a running cluster.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Submit {
    #[clap(flatten)]
    pub config: Config,
    /// Run the job with this same name in the config file.
    #[arg(short, long)]
    pub name: String,
}

/// Establish an `SSH` port-forward from your local machine to the remote
/// cluster's head node.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Connect {
    #[clap(flatten)]
    pub config: Config,
}

/// Launch a native browser window to view the cluster's dashboard.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Dashboard {
    #[clap(flatten)]
    pub config: Config,
}

/// Run a SQL query against the remote cluster.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Sql {
    #[clap(flatten)]
    pub config: Config,
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Config {
    /// Path to configuration file.
    #[arg(short, long, default_value = ".daft.toml")]
    pub config: PathBuf,
}

pub async fn handle() -> anyhow::Result<()> {
    match Cli::parse() {
        Cli::Init(init) => handle_init(init).await,
        Cli::Up(up) => handle_up(up).await,
        Cli::Down(down) => handle_down(down).await,
        Cli::List(list) => handle_list(list).await,
        Cli::Submit(submit) => handle_submit(submit).await,
        Cli::Connect(connect) => handle_connect(connect),
        Cli::Dashboard(dashboard) => handle_dashboard(dashboard),
        Cli::Sql(sql) => handle_sql(sql),
    }
}

async fn assert_authenticated(provider: Option<&processed::Provider>) -> anyhow::Result<()> {
    let (authenticate_with_aws,) = provider.map_or((true,), |provider| match provider {
        processed::Provider::Aws(..) => (true,),
    });

    if authenticate_with_aws {
        assert_authenticated_with_aws().await?;
    };

    Ok(())
}

fn prefix(prefix: &str) -> ColorfulTheme {
    ColorfulTheme {
        prompt_prefix: style(prefix.into()),
        ..Default::default()
    }
}

const NOTEPAD_EMOJI: &str = "📝";
const CLOUD_EMOJI: &str = "🌥️";
const HAMMER_EMOJI: &str = "🔨";
const COMPUTER_EMOJI: &str = "💻";

async fn handle_init(init: Init) -> anyhow::Result<()> {
    assert_file_status(&init.name, Status::DoesNotExist).await?;

    let raw_config = if init.default {
        RawConfig::default()
    } else {
        let name = with_input("Cluster name", &prefix(NOTEPAD_EMOJI), default_name())?;
        let provider = match with_selections::<Provider>("Cloud provider", &prefix(CLOUD_EMOJI))? {
            Provider::Aws(aws_cluster) => {
                let template =
                    with_selections::<AwsTemplateType>("Template", &prefix(HAMMER_EMOJI))?;
                let custom = if template.is_none() {
                    let instance_type = with_input(
                        "Instance type",
                        &prefix(COMPUTER_EMOJI),
                        &*normal_instance_type(),
                    )?;
                    let image_id =
                        with_input("Image ID", &prefix(COMPUTER_EMOJI), &*normal_image_id())?;
                    Some(AwsCustomType {
                        instance_type,
                        image_id,
                    })
                } else {
                    None
                };
                Provider::Aws(AwsCluster {
                    template,
                    custom,
                    ..aws_cluster
                })
            }
        };
        RawConfig {
            package: Package {
                name,
                ..Default::default()
            },
            cluster: Cluster {
                provider,
                ..Default::default()
            },
            ..Default::default()
        }
    };
    let mut file = create_new_file(&init.name).await?;
    let config = toml::to_string_pretty(&raw_config).expect("Serialization should always succeed");
    let config = format!(
        r#"# For a full schema of this configuration file, please visit:
# https://eventual-inc.github.io/daft-launcher
#
# If you notice any bugs, please reach out to me (Raunak Bhagat) via our open Slack workspace, "Distributed Data Community":
# https://join.slack.com/t/dist-data/shared_invite/zt-2ric3mssh-zX08IXaKNeyx8YtqXey41A

{}"#,
        config
    );
    file.write_all(config.as_bytes()).await?;
    println!(
        "Created file at: {}",
        style(format!("`{}`", init.name.display())).cyan(),
    );
    Ok(())
}

fn with_input<S: Into<String>>(
    prompt: &str,
    theme: &ColorfulTheme,
    default: S,
) -> anyhow::Result<StrRef> {
    let value = Input::<String>::with_theme(theme)
        .with_prompt(prompt)
        .default(default.into())
        .interact_text()?
        .into();
    Ok(value)
}

fn with_selections<T: Selectable>(
    prompt: &str,
    theme: &ColorfulTheme,
) -> anyhow::Result<T::Parsed> {
    let options = T::to_options();
    let selection = Select::with_theme(theme)
        .with_prompt(prompt)
        .default(0)
        .items(&options)
        .interact()?;
    let &selection = options
        .get(selection)
        .expect("Index should always be in bounds");
    T::parse(selection)
}

async fn handle_up(up: Up) -> anyhow::Result<()> {
    let (processed_config, ray_config) = read(&up.config.config).await?;
    assert_authenticated(Some(&processed_config.cluster.provider)).await?;
    let cloud_name = match processed_config.cluster.provider {
        processed::Provider::Aws(ref aws_cluster) => {
            assert_non_clashing_cluster_name(
                &processed_config.package.name,
                aws_cluster.region.to_string(),
            )
            .await?;
            if aws_cluster.iam_instance_profile_arn.is_none() {
                log::warn!("You specified no IAM instance profile ARN; this may cause limit your cluster's abilities to interface with auxiliary AWS services");
            }
            format!("`aws (region = {})`", aws_cluster.region)
        }
    };

    run_ray(&ray_config, RaySubcommand::Up, &[]).await?;
    println!(
        "Successfully spun up the cluster {} in your {} cloud",
        style(format!("`{}`", processed_config.package.name)).cyan(),
        style(format!("`{}`", cloud_name)).cyan(),
    );
    Ok(())
}

async fn handle_down(down: Down) -> anyhow::Result<()> {
    let (processed_config, ray_config) = read(&down.config.config).await?;
    match down.name.clone().as_deref() {
        Some(..) => todo!(),
        None => assert_authenticated(Some(&processed_config.cluster.provider)).await?,
    };
    run_ray(&ray_config, RaySubcommand::Down, &[]).await?;
    Ok(())
}

async fn handle_list(list: List) -> anyhow::Result<()> {
    assert_authenticated(None).await?;
    let instances = list_instances("us-west-2").await?;
    let mut table = Table::default();

    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .apply_modifier(UTF8_SOLID_INNER_BORDERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(["Name", "Instance ID", "Status", "IPv4"].map(|header| {
            Cell::new(header)
                .set_alignment(CellAlignment::Center)
                .add_attribute(Attribute::Bold)
        }));

    fn add_instance_to_table(instance: &AwsInstance, table: &mut Table) {
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

    for instance in &instances {
        let is_running = instance
            .state
            .as_ref()
            .map_or(false, |state| *state == InstanceStateName::Running);
        let running_condition = !(list.running && !is_running);
        let regex_condition = list
            .name
            .as_ref()
            .map_or(true, |regex| regex.is_match(&instance.regular_name));
        if running_condition && regex_condition {
            add_instance_to_table(instance, &mut table);
        }
    }

    println!("{}", table);

    Ok(())
}

async fn handle_submit(submit: Submit) -> anyhow::Result<()> {
    let (processed_config, _) = read(&submit.config.config).await?;
    assert_authenticated(Some(&processed_config.cluster.provider)).await?;

    todo!()
}

fn handle_connect(_: Connect) -> anyhow::Result<()> {
    todo!()
}

fn handle_dashboard(_: Dashboard) -> anyhow::Result<()> {
    todo!()
}

fn handle_sql(_: Sql) -> anyhow::Result<()> {
    todo!()
}
