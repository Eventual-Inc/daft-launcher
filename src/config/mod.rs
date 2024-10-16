pub mod custom;
pub mod ray;

use std::{
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::Context;
use tempdir::TempDir;

use crate::config::{
    custom::{CustomConfig, Provider},
    ray::RayConfig,
};

fn process(mut custom_config: CustomConfig) -> anyhow::Result<CustomConfig> {
    match custom_config.cluster.provider {
        Provider::Aws(ref mut aws_cluster) => {}
    }

    todo!()
}

pub fn read_custom(path: &Path) -> anyhow::Result<CustomConfig> {
    let mut file =
        OpenOptions::new().read(true).open(path).with_context(|| {
            format!("No configuration file found at the path {path:?}")
        })?;
    let mut buf = String::new();
    let _ = file
        .read_to_string(&mut buf)
        .with_context(|| format!("Failed to read file {path:?}"))?;
    let custom_config = toml::from_str(&buf)?;
    let custom_config = process(custom_config)?;
    Ok(custom_config)
}

pub fn write_ray(ray: RayConfig) -> anyhow::Result<(TempDir, PathBuf)> {
    todo!()
}

#[cfg(test)]
mod tests {
    use processable_option::ProcessableOption;
    use serde::Deserialize;

    #[test]
    fn test() {
        #[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
        struct Test {
            test: ProcessableOption<Option<bool>>,
        }

        let result = toml::from_str("");
        assert_eq!(
            result,
            Ok(Test {
                test: ProcessableOption::RawNone
            })
        );
    }
}
