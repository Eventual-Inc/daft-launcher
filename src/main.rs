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
    if let Err(error) = handlers::handle().await {
        cli::Cli::command()
            .error(clap::error::ErrorKind::ArgumentConflict, error)
            .exit();
    };
    Ok(())
}
