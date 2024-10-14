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

use std::{io, path};

use thiserror::Error;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
enum Error {
    #[error("{_0}")]
    IoError(#[from] io::Error),

    #[error("{_0}")]
    TomlError(#[from] toml::de::Error),

    #[error("A file with that name already exists at that location: {_0:?}")]
    AlreadyExistsError(path::PathBuf),
}

fn main() -> Result<()> {
    cli::handle()
}
