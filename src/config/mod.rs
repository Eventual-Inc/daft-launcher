pub mod defaults;
pub mod processed;
pub mod raw;
pub mod ray;

use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
    sync::LazyLock,
};

use anyhow::Context;
use processed::ProcessedConfig;
use semver::Version;
use tempdir::TempDir;

use crate::{
    config::{raw::RawConfig, ray::RayConfig},
    utils::create_ray_temporary_file,
    PathRef,
};

static DAFT_LAUNCHER_VERSION: LazyLock<Version> =
    LazyLock::new(|| env!("CARGO_PKG_VERSION").parse().unwrap());

pub trait Selectable {
    type Parsed;

    fn to_options() -> &'static [&'static str];
    fn parse(s: &str) -> anyhow::Result<Self::Parsed>;
}

pub fn read_custom(path: &Path) -> anyhow::Result<(ProcessedConfig, RayConfig)> {
    let mut file =
        OpenOptions::new().read(true).open(path).with_context(|| {
            format!("No configuration file found at the path `{}`; please run `daft init-config` to generate a configuration file", path.display())
        })?;
    let mut buf = String::new();
    let _ = file
        .read_to_string(&mut buf)
        .with_context(|| format!("Failed to read file {path:?}"))?;
    let raw_config: RawConfig = toml::from_str(&buf)?;
    let processed_config: ProcessedConfig = raw_config.try_into()?;
    if !processed_config
        .package
        .daft_launcher_version
        .matches(&*DAFT_LAUNCHER_VERSION)
    {
        anyhow::bail!(
            "The version requirement in the config file located at {:?} (version-requirement {}) is not satisfied by this binary's version (version {})",
            path.display(),
            processed_config.package.daft_launcher_version,
            &*DAFT_LAUNCHER_VERSION,
        );
    }
    let ray_config: RayConfig = processed_config.clone().try_into()?;
    Ok((processed_config, ray_config))
}

pub fn write_ray(ray_config: &RayConfig) -> anyhow::Result<(TempDir, PathRef)> {
    let ray_config =
        serde_yaml::to_string(ray_config).expect("Serialization to yaml should always succeed");
    write_ray_inner(&ray_config)
}

pub fn write_ray_adhoc(
    name: &str,
    provider: &str,
    region: &str,
) -> anyhow::Result<(TempDir, PathRef)> {
    let contents = format!(
        r#"cluster_name: {}
provider:
    type: {}
    region: {}
"#,
        name, provider, region,
    );
    write_ray_inner(&contents)
}

fn write_ray_inner(contents: &str) -> anyhow::Result<(TempDir, PathRef)> {
    let (temp_dir, path, mut file) = create_ray_temporary_file()?;
    log::debug!("Writing ray config to temporary file {}", path.display());
    log::debug!("{}", contents);
    file.write_all(contents.as_bytes())?;
    Ok((temp_dir, path))
}
