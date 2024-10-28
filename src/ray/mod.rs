mod _impl;

use std::sync::Arc;

pub use _impl::RaySubcommand;

use crate::{config::ray::RayConfig, widgets::Spinner};

pub async fn run_ray(
    ray_config: &RayConfig,
    subcommand: _impl::RaySubcommand,
    args: &[&str],
) -> anyhow::Result<()> {
    // # Note
    // Can't use the [`spinner!`] macro here since the internal computations require
    // access to the spinner. This is a special-cased unroll of the macro.
    //
    // If this same pattern arises again, create a new `with_spinner_arced` macro,
    // or something of that like.

    let spinner = Arc::new(Spinner::new("..."));

    {
        let (temp_dir, path) = _impl::write_ray(&ray_config).await?;
        _impl::run_ray(subcommand, path, args, {
            let spinner = spinner.clone();
            move |message| spinner.pause(message)
        })
        .await?;

        // Explicitly deletes the entire temporary directory.
        // The config file that we wrote to inside of there will now be deleted.
        //
        // This should only happen *after* the `ray` command has finished executing.
        drop(temp_dir);
    };

    Arc::try_unwrap(spinner)
        .expect("All other references to `spinner` should be dropped by now, leaving only one")
        .success();

    Ok(())
}
