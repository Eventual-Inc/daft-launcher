use std::{fs, path::PathBuf};

use anyhow::bail;
use clap::{Parser, Subcommand};

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
    /// Initialize a "daft-launcher" configuration file.
    ///
    /// If no path is provided, will create a default ".daft.toml" in the
    /// current working directory.
    Init(Init),
}

#[derive(Debug, Parser, Clone, PartialEq, Eq)]
struct Init {
    /// The path at which to create the config file.
    #[arg(short, long, default_value = ".daft.toml")]
    path: PathBuf,
}

// #[derive(Debug, Parser, Clone, PartialEq, Eq)]
// pub struct Config {
//     /// Path to configuration file.
//     #[arg(short, long, default_value = ".daft.toml")]
//     pub config: PathBuf,
// }

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
    }

    Ok(())
}
