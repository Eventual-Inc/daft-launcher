pub mod custom;
pub mod processable_option;
pub mod ray;

use std::{
    fs,
    io::Read,
    path::{self, PathBuf},
};

use anyhow::Context;
use custom::CustomConfig;
use processable_option::ProcessableOption;

async fn process(
    mut custom_config: CustomConfig,
) -> anyhow::Result<CustomConfig> {
    async fn process_ssh_private_key(
        ssh_private_key: &mut ProcessableOption<Option<PathBuf>>,
    ) -> anyhow::Result<()> {
        ssh_private_key
            .or_else_try_process_async(|| async { todo!() })
            .await?;
        Ok(())
    }

    match custom_config.cluster.provider {
        custom::Provider::Aws(ref mut aws_cluster) => {
            // if aws_cluster.region.is_none() {
            //     aws_cluster.region = Some("us-west-2".to_string());
            // }
            // if aws_cluster.ssh_user.is_none() {
            //     aws_cluster.ssh_user = Some("ubuntu".to_string());
            // }
            // if aws_cluster.iam_instance_profile_arn.is_none() {
            //     aws_cluster.iam_instance_profile_arn =
            //         Some("arn:aws:iam::123456789012:instance-profile/your-instance-profile".to_string());
            // }
            // if aws_cluster.template.is_none() {
            //     aws_cluster.template = Some(custom::AwsTemplateType::Normal);
            // }
        }
    }

    todo!()
}

pub async fn read_custom(
    path: &path::Path,
) -> anyhow::Result<custom::CustomConfig> {
    let mut file =
        fs::OpenOptions::new()
            .read(true)
            .open(path)
            .with_context(|| {
                format!("No configuration file found at the path {path:?}")
            })?;
    let mut buf = String::new();
    let _ = file
        .read_to_string(&mut buf)
        .with_context(|| format!("Failed to read file {path:?}"))?;
    let custom_config = toml::from_str(&buf)?;
    let custom_config = process(custom_config).await?;
    Ok(custom_config)
}

pub fn write_ray(
    ray: ray::RayConfig,
) -> anyhow::Result<(tempdir::TempDir, path::PathBuf)> {
    todo!()
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::config::processable_option;

    #[test]
    fn test() {
        #[derive(Debug, Deserialize)]
        struct Test {
            #[serde(default = "processable_option::ProcessableOption::empty")]
            test: processable_option::ProcessableOption<Option<bool>>,
        }

        let result = toml::from_str::<Test>("");
        dbg!(&result);
    }
}
