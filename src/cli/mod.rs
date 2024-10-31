mod _assert;
mod _impl;

use std::path::PathBuf;

use clap::Parser;
use regex::Regex;

use crate::{config::read, ArcStrRef};

#[derive(Parser)]
#[command(version, about = env!("CARGO_PKG_DESCRIPTION"))]
pub enum Cli {
    Init(Init),
    Up(Up),
    Down(Down),
    List(List),
    Submit(Submit),
    Connect(Connect),
    Sql(Sql),
    Export(Export),
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
    /// Only list the head nodes of clusters.
    #[arg(long)]
    pub head: bool,
    /// Only list the clusters that match this regex.
    #[arg(short, long)]
    pub name: Option<Regex>,
}

/// Submit a job to a running cluster.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Submit {
    #[clap(flatten)]
    pub config: Config,
    /// Run the job with this same name in the config file.
    pub name: ArcStrRef,
}

/// Establish an `SSH` port-forward from your local machine to the remote
/// cluster's head node.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Connect {
    #[clap(flatten)]
    pub config: Config,
    /// Stop the Ray dashboard from opening upon connection to the remote cluster.
    #[arg(short, long)]
    pub no_dashboard: bool,
}

/// Run a SQL query against the remote cluster.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Sql {
    #[clap(flatten)]
    pub config: Config,
    /// The SQL query to execute on the remote cluster.
    #[arg(trailing_var_arg = true)]
    args: Vec<ArcStrRef>,
}

/// Exports the Ray YAML file that is generated internally to interface with the
/// Ray CLI.
///
/// This should largely be used for escape-hatching + debugging. Most users
/// should not have to interact with this feature.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct Export {
    #[clap(flatten)]
    pub config: Config,
    /// The path for which to write the generated Ray YAML file into.
    #[arg(short, long, default_value = ".ray.yaml")]
    pub name: PathBuf,
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
        Cli::Connect(connect) => handle_connect(connect).await,
        Cli::Sql(sql) => handle_sql(sql).await,
        Cli::Export(export) => handle_export(export).await,
    }
}

async fn handle_init(init: Init) -> anyhow::Result<()> {
    _assert::assert_init(&init).await?;
    _impl::handle_init(init).await?;
    Ok(())
}

async fn handle_up(up: Up) -> anyhow::Result<()> {
    let (processed_config, ray_config) = read(&up.config.config).await?;
    _assert::assert_up(&processed_config).await?;
    _impl::handle_up(processed_config, ray_config).await?;
    Ok(())
}

async fn handle_down(down: Down) -> anyhow::Result<()> {
    let (processed_config, ray_config) = read(&down.config.config).await?;
    _assert::assert_down(&down, &processed_config).await?;
    _impl::handle_down(processed_config, ray_config).await?;
    Ok(())
}

async fn handle_list(list: List) -> anyhow::Result<()> {
    _assert::assert_list().await?;
    _impl::handle_list(list).await?;
    Ok(())
}

async fn handle_submit(submit: Submit) -> anyhow::Result<()> {
    let (processed_config, ray_config) = read(&submit.config.config).await?;
    _assert::assert_submit_and_connect_and_sql(&processed_config).await?;
    _impl::handle_submit(submit, processed_config, ray_config).await?;
    Ok(())
}

async fn handle_connect(connect: Connect) -> anyhow::Result<()> {
    let (processed_config, _) = read(&connect.config.config).await?;
    _assert::assert_submit_and_connect_and_sql(&processed_config).await?;
    _impl::handle_connect(processed_config, connect.no_dashboard).await?;
    Ok(())
}

async fn handle_sql(sql: Sql) -> anyhow::Result<()> {
    let (processed_config, ray_config) = read(&sql.config.config).await?;
    _assert::assert_submit_and_connect_and_sql(&processed_config).await?;
    _impl::handle_sql(processed_config, ray_config, sql.args).await?;
    Ok(())
}

async fn handle_export(export: Export) -> anyhow::Result<()> {
    _assert::assert_export(&export).await?;
    let (_, ray_config) = read(&export.config.config).await?;
    _impl::handle_export(&export.name, ray_config).await?;
    Ok(())
}
