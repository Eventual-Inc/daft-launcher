use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
};

use clap::Parser;
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use tempdir::TempDir;

use crate::{
    aws::{assert_is_authenticated_with_aws, list_instances, AwsInstance},
    config::{
        defaults::{normal_image_id, normal_instance_type},
        processed::{self, ProcessedConfig},
        raw::{
            default_name, AwsCluster, AwsCustomType, AwsTemplateType, Cluster, Package, Provider,
            RawConfig,
        },
        read_custom, write_ray, Selectable,
    },
    utils::{assert_file_existence_status, create_new_file, is_debug, path_to_str},
    widgets::Spinner,
    ArcStrRef, PathRef, StrRef,
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
/// If nothing is provided, a configuration file name `.daft.toml` will be created in the current directory.
/// These configuration files are the entry point into the daft launcher utility tool.
/// They contain all the necessary information to spin up a cluster, submit jobs, and more.
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

/// List all clusters in each cloud provider.
///
/// This will list all of the clusters, regardless of their state (i.e., running, stopped, etc.).
/// You can filter the results by appending any of the supported flags.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct List {
    /// Only list the clusters that are running.
    #[arg(short, long)]
    pub running: bool,
}

/// Submit a job to a running cluster.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Submit {
    #[clap(flatten)]
    pub config: Config,
}

/// Establish an `SSH` port-forward from your local machine to the remote cluster's head node.
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
        Cli::Init(init) => handle_init(init),
        Cli::Up(up) => handle_up(up).await,
        Cli::Down(down) => handle_down(down).await,
        Cli::List(list) => handle_list(list).await,
        Cli::Submit(submit) => handle_submit(submit),
        Cli::Connect(connect) => handle_connect(connect),
        Cli::Dashboard(dashboard) => handle_dashboard(dashboard),
        Cli::Sql(sql) => handle_sql(sql),
    }
}

async fn assert_is_authenticated_with_cloud_provider_in_config(
    processed_config: &ProcessedConfig,
) -> anyhow::Result<()> {
    match processed_config.cluster.provider {
        processed::Provider::Aws(..) => assert_is_authenticated_with_aws().await,
    }
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

fn handle_init(init: Init) -> anyhow::Result<()> {
    assert_file_existence_status(&init.name, false)?;
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
    let mut file = create_new_file(&init.name)?;
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
    file.write_all(config.as_bytes())?;
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

fn format_aws_instance_names(name: &str, instances: &[AwsInstance]) -> String {
    let mut joined_names = String::new();
    for (index, instance) in instances.iter().enumerate() {
        if index != 0 {
            joined_names.push_str(&style(", ").green().to_string());
        }
        let styled_name = if instance.name_equals_ray_name(name) {
            style(&instance.ray_name).bold()
        } else {
            style(&instance.ray_name)
        }
        .green()
        .to_string();
        joined_names.push_str(&styled_name);
    }
    joined_names
}

async fn handle_up(up: Up) -> anyhow::Result<()> {
    let (processed_config, ray_config) = read_custom(&up.config.config)?;
    assert_is_authenticated_with_cloud_provider_in_config(&processed_config).await?;
    let (temp_dir, path) = write_ray(&ray_config)?;
    let cloud_name = match processed_config.cluster.provider {
        processed::Provider::Aws(ref aws_cluster) => {
            let spinner = Spinner::new("Validating cluster specs");
            let instances = list_instances(aws_cluster).await?;
            let mut instance_name_already_exists = false;
            for instance in &instances {
                if instance.name_equals_ray_name(&processed_config.package.name) {
                    instance_name_already_exists = true;
                }
            }
            if instance_name_already_exists {
                spinner.fail();
                let names = format_aws_instance_names(&processed_config.package.name, &instances);
                anyhow::bail!(r#"An instance with the name "{}" already exists in that specified region; please choose a different name
Instance names: {}
{}"#,
                    processed_config.package.name,
                    names,
                    style("*Note that Ray prepends `ray-` before and appends `-head` after the name of your cluster").red(),
                );
            };
            spinner.success();
            if aws_cluster.iam_instance_profile_arn.is_none() {
                log::warn!("You specified no IAM instance profile ARN; this may cause limit your cluster's abilities to interface with auxiliary AWS services");
            }
            format!("`aws (region = {})`", aws_cluster.region)
        }
    };
    run_ray_command(temp_dir, path, "up", None, "Spinning up your cluster...")?;
    println!(
        "Successfully spun the cluster {} in your {} cloud",
        style(format!("`{}`", processed_config.package.name)).cyan(),
        style(format!("`{}`", cloud_name)).cyan(),
    );
    Ok(())
}

async fn handle_down(down: Down) -> anyhow::Result<()> {
    let (processed_config, ray_config) = read_custom(&down.config.config)?;
    let (temp_dir, path) = match down.name.clone() {
        Some(..) => todo!(),
        None => {
            assert_is_authenticated_with_cloud_provider_in_config(&processed_config).await?;
            write_ray(&ray_config)
        }
    }?;
    run_ray_command(
        temp_dir,
        path,
        "down",
        None,
        "Spinning down your cluster...",
    )?;
    Ok(())
}

async fn handle_list(list: List) -> anyhow::Result<()> {
    assert_is_authenticated_with_aws().await?;
    todo!()
}

fn handle_submit(_: Submit) -> anyhow::Result<()> {
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

fn run_ray_command(
    temp_dir: TempDir,
    path: PathRef,
    sub_command: &str,
    args: Option<&[&str]>,
    spinner_message: &'static str,
) -> anyhow::Result<()> {
    let args = args.unwrap_or(&[]);
    let spinner = Spinner::new(spinner_message);

    let mut child = Command::new("ray")
        .env("PYTHONUNBUFFERED", "1")
        .arg(sub_command)
        .arg("-y")
        .arg(path_to_str(path.as_os_str())?)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let child_stdout = BufReader::new(child.stdout.take().expect("Stdout should always exist"));

    thread::spawn({
        let spinner = spinner.clone();
        move || {
            for line in child_stdout
                .lines()
                .map(|line| line.expect("Reading line from child process should always succeed"))
            {
                spinner.pause(&line);
            }
        }
    });

    let exit_status = child.wait()?;

    // Explicitly deletes the entire temporary directory.
    // The config file that we wrote to inside of there will now be deleted.
    //
    // This should only happen *after* the `ray` command has finished executing.
    drop(temp_dir);

    if exit_status.success() {
        spinner.success();
        Ok(())
    } else {
        spinner.fail();
        let child_stderr = BufReader::new(child.stderr.take().expect("Stderr should always exist"));

        if is_debug() {
            let full_child_backtrace = child_stderr
                .lines()
                .flatten()
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::bail!(
                "Command failed with exit status: {}\n{}",
                exit_status,
                full_child_backtrace,
            )
        } else {
            let mut last_line = None;
            for line in child_stderr.lines().flatten() {
                last_line = Some(line);
            }
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
