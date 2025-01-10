use std::{fs, path::PathBuf, sync::Arc};

use anyhow::bail;
use clap::{Parser, Subcommand};
use semver::{Version, VersionReq};
use serde::Deserialize;
use serde::Serialize;

type StrRef = Arc<str>;

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Command {
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

    /// Spin up a new cluster.
    ///
    /// If no configuration file path is provided, this will try to find a
    /// ".daft.toml" in the current working directory and use that.
    Up(Up),
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Init {
    /// The path at which to create the config file.
    #[arg(short, long, default_value = ".daft.toml")]
    path: PathBuf,
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Up {
    #[clap(flatten)]
    config: ConfigPath,
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct ConfigPath {
    /// Path to configuration file.
    #[arg(short, long, default_value = ".daft.toml")]
    path: PathBuf,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
struct RawConfig {
    setup: Setup,
    #[serde(default)]
    run: Run,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
struct Setup {
    name: StrRef,
    #[serde(deserialize_with = "parse_version_req")]
    version: VersionReq,
    provider: Provider,
}

fn parse_version_req<'de, D>(deserializer: D) -> Result<VersionReq, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: StrRef = Deserialize::deserialize(deserializer)?;
    let version_req = raw
        .parse::<VersionReq>()
        .map_err(serde::de::Error::custom)?;
    let current_version = env!("CARGO_PKG_VERSION").parse::<Version>().unwrap();
    if version_req.matches(&current_version) {
        Ok(version_req)
    } else {
        Err(serde::de::Error::custom(format!("You're running daft-launcher version {current_version}, but your configuration file requires version {version_req}")))
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Provider {
    Aws,
}

#[derive(Default, Debug, Deserialize, Clone, PartialEq, Eq)]
struct Run {
    #[serde(default)]
    pre_setup: Vec<StrRef>,
    #[serde(default)]
    post_setup: Vec<StrRef>,
}

#[derive(Default, Debug, Serialize, Clone, PartialEq, Eq)]
struct RayConfig;

fn main() -> anyhow::Result<()> {
    let command = Command::parse();
    match command.sub_command {
        SubCommand::Init(Init { path }) => {
            if path.exists() {
                bail!("The path '{path:?}' already exists; the path given must point to a new location on your filesystem");
            };
            let contents = include_str!("default.toml");
            let contents = contents.replace("<VERSION>", env!("CARGO_PKG_VERSION"));
            fs::write(path, contents)?;
        }
        SubCommand::Up(Up { config }) => {
            let contents = fs::read_to_string(&config.path)?;
            let raw_config = toml::from_str::<RawConfig>(&contents)?;
            dbg!(raw_config);
        }
    }

    Ok(())
}
