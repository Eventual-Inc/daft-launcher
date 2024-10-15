pub mod ray;

use std::path;

use serde::Deserialize;

use crate::utils;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct CustomConfig {
    pub package: Package,
    pub cluster: Cluster,
    #[serde(rename = "job", default)]
    pub jobs: Vec<Job>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Package {
    pub daft_launcher_version: semver::Version,
    pub name: String,
    pub python_version: Option<semver::VersionReq>,
    pub ray_version: Option<semver::VersionReq>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Cluster {
    #[serde(flatten)]
    pub provider: Provider,
    #[serde(default = "utils::default_number_of_workers")]
    pub number_of_workers: usize,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub pre_setup_commands: Vec<String>,
    #[serde(default)]
    pub post_setup_commands: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "provider")]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Aws(AwsCluster),
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct AwsCluster {
    #[serde(default = "utils::default_region")]
    pub region: String,
    #[serde(default)]
    pub ssh_user: Option<String>,
    #[serde(default)]
    pub ssh_private_key: Option<path::PathBuf>,
    #[serde(default)]
    pub iam_instance_profile_arn: Option<String>,

    pub template: Option<AwsTemplate>,
    #[serde(flatten)]
    pub custom: Option<AwsCustom>,
}

#[derive(Default, Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AwsTemplate {
    #[default]
    Light,
    Normal,
    Gpus,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct AwsCustom {
    #[serde(default)]
    pub image_id: Option<String>,
    #[serde(default)]
    pub instance_type: Option<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Job {
    pub name: String,
    pub working_dir: path::PathBuf,
    pub command: String,
}
