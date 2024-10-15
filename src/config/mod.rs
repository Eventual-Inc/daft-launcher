pub mod custom;
pub mod ray;

use std::{fs, future, io::Read, path};

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
#[must_use]
pub enum ProcessState<T> {
    Unprocessed(Option<T>),
    #[serde(skip_serializing)]
    Processed(T),
}

impl<T: Default> Default for ProcessState<T> {
    fn default() -> Self {
        Self::Unprocessed(Some(T::default()))
    }
}

impl<T> ProcessState<T> {
    pub fn empty() -> Self {
        Self::Unprocessed(None)
    }

    pub fn try_process(
        self,
        f: impl FnOnce(Option<T>) -> anyhow::Result<T>,
    ) -> anyhow::Result<Self> {
        match self {
            Self::Unprocessed(t) => Ok(Self::Processed(f(t)?)),
            Self::Processed(..) => panic!(),
        }
    }

    pub async fn try_process_async<
        F: future::Future<Output = anyhow::Result<T>>,
    >(
        self,
        f: impl FnOnce(Option<T>) -> F,
    ) -> anyhow::Result<Self> {
        match self {
            Self::Unprocessed(t) => Ok(Self::Processed(f(t).await?)),
            Self::Processed(..) => panic!(),
        }
    }

    pub fn as_processed(&self) -> &T {
        match self {
            Self::Unprocessed(..) => panic!(),
            Self::Processed(t) => t,
        }
    }

    pub fn as_processed_mut(&mut self) -> &mut T {
        match self {
            Self::Unprocessed(..) => panic!(),
            Self::Processed(t) => t,
        }
    }
}

async fn process(
    mut custom_config: custom::CustomConfig,
) -> anyhow::Result<custom::CustomConfig> {
    async fn process_ssh_private_key(
        process_state: ProcessState<Option<path::PathBuf>>,
    ) -> anyhow::Result<ProcessState<Option<path::PathBuf>>> {
        let process_state = process_state
            .try_process_async(|_| async { todo!() })
            .await?;
        Ok(process_state)
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
