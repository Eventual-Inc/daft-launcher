use clap::Parser;
use clap::Subcommand;

#[derive(Debug, Clone, Parser)]
struct Commands {
    #[command(subcommand)]
    sub_commands: SubCommands,

    /// Enable verbose printing
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbosity: u8,
}

#[derive(Debug, Clone, Subcommand)]
enum SubCommands {
    Init(Init),
}

#[derive(Debug, Clone, Parser)]
struct Init;

fn main() {
    let commans = Commands::parse();
}
