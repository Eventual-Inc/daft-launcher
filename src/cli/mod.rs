mod handlers;

use std::path;

use clap::Parser;

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
    name: path::PathBuf,

    /// Run in interactive mode
    #[arg(short, long, default_value = "false")]
    interactive: bool,
}

#[derive(Parser)]
pub struct Up {
    #[clap(flatten)]
    config: Config,
}

#[derive(Parser)]
pub struct Down {
    #[clap(flatten)]
    config: Config,

    /// Name of the cluster
    #[arg(short, long, value_name = "NAME")]
    name: String,
}

#[derive(Parser)]
pub struct Submit {
    #[clap(flatten)]
    config: Config,
}

#[derive(Parser)]
pub struct Connect {
    #[clap(flatten)]
    config: Config,
}

#[derive(Parser)]
pub struct Dashboard {
    #[clap(flatten)]
    config: Config,
}

#[derive(Parser)]
pub struct Sql {
    #[clap(flatten)]
    config: Config,
}

#[derive(Parser, Clone)]
pub struct Config {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE", default_value = ".daft.toml")]
    config: path::PathBuf,
}

pub fn handle() -> crate::Result<()> {
    match Cli::parse() {
        Cli::InitConfig(init_config) => handlers::handle_init_config(init_config),
        Cli::Up(up) => handlers::handle_up(up),
        Cli::Down(down) => handlers::handle_down(down),
        Cli::Submit(submit) => handlers::handle_submit(submit),
        Cli::Connect(connect) => handlers::handle_connect(connect),
        Cli::Dashboard(dashboard) => handlers::handle_dashboard(dashboard),
        Cli::Sql(sql) => handlers::handle_sql(sql),
    }
}
