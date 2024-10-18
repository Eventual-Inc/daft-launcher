pub mod processed;
pub mod raw;
pub mod ray;

use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};

use anyhow::Context;
use processed::ProcessedConfig;
use tempdir::TempDir;

use crate::{
    config::{raw::RawConfig, ray::RayConfig},
    utils::create_ray_temporary_file,
    PathRef,
};

pub fn read_custom(
    path: &Path,
) -> anyhow::Result<(ProcessedConfig, RayConfig)> {
    let mut file =
        OpenOptions::new().read(true).open(path).with_context(|| {
            format!("No configuration file found at the path {path:?}")
        })?;
    let mut buf = String::new();
    let _ = file
        .read_to_string(&mut buf)
        .with_context(|| format!("Failed to read file {path:?}"))?;
    let raw_config: RawConfig = toml::from_str(&buf)?;
    let processed_config: ProcessedConfig = raw_config.try_into()?;
    let ray_config: RayConfig = processed_config.clone().try_into()?;
    Ok((processed_config, ray_config))
}

pub fn write_ray(ray_config: &RayConfig) -> anyhow::Result<(TempDir, PathRef)> {
    let (temp_dir, path, mut file) = create_ray_temporary_file()?;
    log::info!("Writing ray config {ray_config:?} to temporary file {path:?}");
    let ray_config = serde_yaml::to_string(ray_config)
        .expect("Serialization to yaml should always succeed");
    file.write_all(ray_config.as_bytes())?;
    Ok((temp_dir, path))
}

pub fn write_ray_adhoc(
    name: &str,
    r#type: &str,
    region: &str,
) -> anyhow::Result<(TempDir, PathRef)> {
    let contents = format!(
        r#"cluster_name: {}
provider:
    type: {}
    region: {}
"#,
        name, r#type, region,
    );
    let (temp_dir, path, mut file) = create_ray_temporary_file()?;
    log::info!("Writing ray config {contents:?} to temporary file {path:?}");
    file.write_all(contents.as_bytes())?;
    Ok((temp_dir, path))
}
