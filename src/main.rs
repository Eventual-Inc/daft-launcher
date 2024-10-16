macro_rules! path_from_root {
    ($($segment:literal) / *) => {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            $(
                concat!("/", $segment)
            ),*
        )
    };
}

mod cli;
mod config;
mod handlers;
mod processable_option;
mod utils;

use clap::CommandFactory;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(error) = handlers::handle().await {
        cli::Cli::command()
            .error(clap::error::ErrorKind::ArgumentConflict, error)
            .exit();
    };
    Ok(())
}
