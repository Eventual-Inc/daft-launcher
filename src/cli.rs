use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::LazyLock,
};

use clap::Parser;
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use semver::Version;
use tempdir::TempDir;

use crate::{
    config::{
        processed::ProcessedConfig,
        raw::{
            default_name, AwsCluster, AwsTemplateType, Cluster, Package,
            Provider, RawConfig,
        },
        read_custom, write_ray, write_ray_adhoc, Selectable,
    },
    utils::{
        assert_file_doesnt_exist, assert_is_authenticated_with_aws,
        create_new_file, path_to_str,
    },
    ArcStrRef, PathRef, StrRef,
};

#[derive(Parser)]
#[command(version, about = env!("CARGO_PKG_DESCRIPTION"))]
pub enum Cli {
    InitConfig(InitConfig),
    Up(Up),
    Down(Down),
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
pub struct InitConfig {
    /// Name of the configuration file (can be specified as a path).
    #[arg(short, long, value_name = "NAME", default_value = ".daft.toml")]
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
    /// Name of the cluster to spin down
    #[arg(short, long, value_name = "NAME")]
    pub name: Option<ArcStrRef>,
    /// Type of cloud provider which contains the cluster to spin down
    #[arg(short, long, value_name = "TYPE")]
    pub r#type: Option<ArcStrRef>,
    /// Region of the cluster to spin down
    #[arg(short, long, value_name = "REGION")]
    pub region: Option<ArcStrRef>,
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
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE", default_value = ".daft.toml")]
    pub config: PathBuf,
}

static DAFT_LAUNCHER_VERSION: LazyLock<Version> =
    LazyLock::new(|| env!("CARGO_PKG_VERSION").parse().unwrap());

// static THEME: LazyLock<ColorfulTheme> = LazyLock::new(|| ColorfulTheme {
//     prompt_prefix: style(":)".into()),
//     ..Default::default()
// });

pub async fn handle() -> anyhow::Result<()> {
    match Cli::parse() {
        Cli::InitConfig(init_config) => handle_init_config(init_config),
        Cli::Up(up) => {
            assert_is_authenticated_with_aws().await?;
            handle_up(up)
        }
        Cli::Down(down) => {
            assert_is_authenticated_with_aws().await?;
            handle_down(down)
        }
        Cli::Submit(submit) => handle_submit(submit),
        Cli::Connect(connect) => handle_connect(connect),
        Cli::Dashboard(dashboard) => handle_dashboard(dashboard),
        Cli::Sql(sql) => handle_sql(sql),
    }
}

fn prefix(prefix: &str) -> ColorfulTheme {
    ColorfulTheme {
        prompt_prefix: style(prefix.into()),
        ..Default::default()
    }
}

const NAME_EMOJI: &str = "📝";
const CLOUD_PROVIDER_EMOJI: &str = "🌥️";
const TEMPLATE_EMOJI: &str = "🔨";

fn handle_init_config(init_config: InitConfig) -> anyhow::Result<()> {
    assert_file_doesnt_exist(&init_config.name)?;
    let raw_config = if init_config.default {
        RawConfig::default()
    } else {
        let name: StrRef = Input::<String>::with_theme(&prefix(NAME_EMOJI))
            .with_prompt("Cluster name")
            .default(default_name())
            .interact_text()?
            .into();
        let provider = match get_selections::<Provider>(
            "Cloud provider",
            &prefix(CLOUD_PROVIDER_EMOJI),
        )? {
            Provider::Aws(aws_cluster) => {
                let template = get_selections::<AwsTemplateType>(
                    "Template",
                    &prefix(TEMPLATE_EMOJI),
                )?;
                Provider::Aws(AwsCluster {
                    template: Some(template),
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
    let mut file = create_new_file(&init_config.name)?;
    let config = toml::to_string_pretty(&raw_config)
        .expect("Serialization should always succeed");
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
    Ok(())
}

fn get_selections<T: Selectable>(
    prompt: &str,
    theme: &ColorfulTheme,
) -> anyhow::Result<T> {
    let options = T::to_options();
    let selection = Select::with_theme(theme)
        .with_prompt(prompt)
        .default(0)
        .items(&options)
        .interact()?;
    let selection = options
        .get(selection)
        .expect("Index should always be in bounds")
        .parse()?;
    Ok(selection)
}

fn handle_up(up: Up) -> anyhow::Result<()> {
    let (processed_config, ray_config) = read_custom(&up.config.config)?;
    assert_matching_daft_launcher_versions(
        &up.config.config,
        &processed_config,
    )?;

    let (temp_dir, path) = write_ray(&ray_config)?;
    run_ray_command(temp_dir, path, "up", None)
}

fn handle_down(down: Down) -> anyhow::Result<()> {
    let (processed_config, ray_config) = read_custom(&down.config.config)?;
    assert_matching_daft_launcher_versions(
        &down.config.config,
        &processed_config,
    )?;

    match (down.name, down.r#type, down.region) {
        (Some(name), Some(r#type), Some(region)) => {
            let (temp_dir, path) =
                write_ray_adhoc(&name, &r#type, &region)?;
            run_ray_command(temp_dir, path, "down", None)
        }
        (None, None, None) => {
            let (temp_dir, path) = write_ray(&ray_config)?;
            run_ray_command(temp_dir, path, "down", None)
        }
        _ => anyhow::bail!("You must provide all three arguments to spin down a cluster: name, type, and region"),
    }
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
) -> anyhow::Result<()> {
    let args = args.unwrap_or(&[]);
    let _ = Command::new("ray")
        .arg(sub_command)
        .arg(path_to_str(path.as_os_str())?)
        .args(args)
        .spawn()?
        .wait()?;

    // Explicitly deletes the entire temporary directory.
    // The config file that we wrote to inside of there will now be deleted.
    drop(temp_dir);

    Ok(())
}

fn assert_matching_daft_launcher_versions(
    config_path: &Path,
    processed_config: &ProcessedConfig,
) -> anyhow::Result<()> {
    if processed_config
        .package
        .daft_launcher_version
        .matches(&DAFT_LAUNCHER_VERSION)
    {
        Ok(())
    } else {
        anyhow::bail!("The version requirement in the config file located at {:?} (version-requirement {}) is not satisfied by this binary's version (version {})", config_path, processed_config.package.daft_launcher_version, &*DAFT_LAUNCHER_VERSION)
    }
}
