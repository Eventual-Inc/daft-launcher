use std::path::PathBuf;

use clap::Parser;

use crate::ArcStrRef;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub enum Cli {
    InitConfig(InitConfig),
    Up(Up),
    Down(Down),
    Submit(Submit),
    Connect(Connect),
    Dashboard(Dashboard),
    Sql(Sql),
}

#[derive(Parser)]
pub struct InitConfig {
    /// Name of the configuration file (can be specified as a path)
    #[arg(short, long, value_name = "NAME", default_value = ".daft.toml")]
    pub name: PathBuf,

    /// Run in interactive mode
    #[arg(short, long, default_value = "false")]
    pub interactive: bool,
}

#[derive(Parser)]
pub struct Up {
    #[clap(flatten)]
    pub config: Config,
}

#[derive(Parser)]
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

#[derive(Parser)]
pub struct Submit {
    #[clap(flatten)]
    pub config: Config,
}

#[derive(Parser)]
pub struct Connect {
    #[clap(flatten)]
    pub config: Config,
}

#[derive(Parser)]
pub struct Dashboard {
    #[clap(flatten)]
    pub config: Config,
}

#[derive(Parser)]
pub struct Sql {
    #[clap(flatten)]
    pub config: Config,
}

#[derive(Parser, Clone)]
pub struct Config {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE", default_value = ".daft.toml")]
    pub config: PathBuf,
}
