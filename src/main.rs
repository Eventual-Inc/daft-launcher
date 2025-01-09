use std::{fs, path::PathBuf, sync::Arc};

use anyhow::bail;
use clap::{Parser, Subcommand};
use semver::{Version, VersionReq};
use serde::Deserialize;

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
    /// Initialize a "daft launcher" configuration file.
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
    #[serde(deserialize_with = "deserialize_version")]
    version: VersionReq,
    provider: Provider,
}

fn deserialize_version<'de, D>(deserializer: D) -> Result<VersionReq, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: StrRef = Deserialize::deserialize(deserializer)?;
    raw.parse().map_err(serde::de::Error::custom)
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

fn main() -> anyhow::Result<()> {
    let command = Command::parse();
    match command.sub_command {
        SubCommand::Init(Init { path }) => {
            if path.exists() {
                bail!("The path '{:?}' already exists; the path given must point to a new location on your filesystem", path);
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
