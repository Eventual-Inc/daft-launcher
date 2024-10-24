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

use std::{env, path::Path, rc::Rc, sync::Arc};

use clap::CommandFactory;

pub type ArcStrRef = Arc<str>;
pub type StrRef = Rc<str>;
pub type PathRef = Rc<Path>;

pub fn path_ref<'a>(path: impl AsRef<Path>) -> PathRef {
    Rc::from(Path::new(path.as_ref()))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::try_init().ok();
    log::debug!("daft launcher - {}", env!("CARGO_PKG_VERSION"));
    if let Some("0") | None = env::var("RUST_BACKTRACE").ok().as_deref() {
        log::debug!("Backtraces are disabled; to enable them, rerun with `RUST_BACKTRACE=1`");
    };
    if let Err(error) = cli::handle().await {
        log::debug!("Error: {}", error);
        log::debug!("Backtrace: {}", error.backtrace());
        cli::Cli::command()
            .error(clap::error::ErrorKind::ArgumentConflict, error)
            .exit();
    };
    Ok(())
}
