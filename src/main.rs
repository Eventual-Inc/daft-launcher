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

macro_rules! spinner {
    {
        $msg:expr
        , $e:expr
        $(,)?
    } => {{
        use crate::widgets::Spinner;

        let spinner = Spinner::new($msg);
        let output = $e;
        spinner.success();
        output
    }};
}

mod aws;
mod cli;
mod config;
mod ray;
mod utils;
mod widgets;

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
    if let None = env::var("RUST_LOG").ok() {
        env::set_var("RUST_LOG", "warn");
    };
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
