use std::{
    fs,
    io::{self, Read, Write},
    path,
};

use anyhow::Context;

use crate::config::{self, ray};

pub async fn is_authenticated_with_aws() -> bool {
    // let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
    //     .region(aws_config::meta::region::RegionProviderChain::default_provider())
    //     .load()
    //     .await;
    // let client = aws_sdk_sts::Client::new(&sdk_config);
    // client.get_caller_identity().send().await.is_ok()
    true
}

pub async fn assert_is_authenticated_with_aws() -> anyhow::Result<()> {
    if is_authenticated_with_aws().await {
        Ok(())
    } else {
        anyhow::bail!("You are not signed in to AWS; please sign in first")
    }
}

pub fn read_custom_config(path: &path::Path) -> anyhow::Result<config::CustomConfig> {
    let mut file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .with_context(|| format!("No configuration file found at the path {path:?}"))?;
    let mut buf = String::new();
    let _ = file
        .read_to_string(&mut buf)
        .with_context(|| format!("Failed to read file {path:?}"))?;
    let custom_config = toml::from_str(&buf)?;
    Ok(custom_config)
}

pub fn write_ray_config(
    ray_config: &ray::RayConfig,
) -> anyhow::Result<(tempdir::TempDir, path::PathBuf)> {
    let temp_dir = tempdir::TempDir::new("ray_config")
        .expect("Creation of temporary directory should always succeed");
    let path = temp_dir.path().join("ray.yaml");
    let ray_config =
        serde_yaml::to_string(ray_config).expect("Serialization should always succeed");
    create_new_file(&path)
        .expect("Creating new file in temporary directory should always succeed")
        .write_all(ray_config.as_bytes())
        .expect("Writing to file should always succeed");
    Ok((temp_dir, path))
}

pub fn create_new_file(path: &path::Path) -> anyhow::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => anyhow::anyhow!("The file {:?} already exists", path),
            _ => error.into(),
        })
}
