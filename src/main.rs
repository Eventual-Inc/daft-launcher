#[cfg(test)]
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
mod utils;

use std::{path::Path, rc::Rc, sync::Arc};

use clap::CommandFactory;

pub type ArcStrRef = Arc<str>;
pub type StrRef = Rc<str>;
pub type PathRef = Rc<Path>;

pub fn path_ref<'a>(x: impl AsRef<Path>) -> PathRef {
    Rc::from(Path::new(x.as_ref()))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::try_init().ok();
    log::debug!("daft launcher - {}", env!("CARGO_PKG_VERSION"));
    if let Err(error) = cli::handle().await {
        log::error!("Error: {}", error);
        log::error!("Backtrace: {}", error.backtrace());
        cli::Cli::command()
            .error(clap::error::ErrorKind::ArgumentConflict, error)
            .exit();
    };
    Ok(())
}
