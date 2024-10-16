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

#[cfg(test)]
macro_rules! read_toml {
    ($($t:tt)*) => {{
        let contents = include_str!(path_from_root!($($t)*));
        toml::from_str(contents).unwrap()
    }};
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
