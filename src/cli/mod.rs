mod _impl;

use std::path::PathBuf;

use clap::Parser;
use regex::Regex;

use crate::{
    aws::{
        assert_authenticated as assert_authenticated_with_aws, assert_non_clashing_cluster_name,
    },
    config::{processed, read},
    utils::{assert_file_status, Status},
    ArcStrRef,
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

async fn handle_init(init: Init) -> anyhow::Result<()> {
    assert_file_status(&init.name, Status::DoesNotExist).await?;

    _impl::handle_init(init).await
}

async fn handle_up(up: Up) -> anyhow::Result<()> {
    let (processed_config, ray_config) = read(&up.config.config).await?;
    assert_authenticated(Some(&processed_config.cluster.provider)).await?;
    match processed_config.cluster.provider {
        processed::Provider::Aws(ref aws_cluster) => {
            assert_non_clashing_cluster_name(
                &processed_config.package.name,
                aws_cluster.region.to_string(),
            )
            .await?;
        }
    };

    _impl::handle_up(processed_config, ray_config).await
}

async fn handle_down(down: Down) -> anyhow::Result<()> {
    let (processed_config, ray_config) = read(&down.config.config).await?;
    match down.name.clone().as_deref() {
        Some(..) => todo!(),
        None => assert_authenticated(Some(&processed_config.cluster.provider)).await?,
    };

    _impl::handle_down(ray_config).await
}

async fn handle_list(list: List) -> anyhow::Result<()> {
    assert_authenticated(None).await?;

    _impl::handle_list(list).await
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
