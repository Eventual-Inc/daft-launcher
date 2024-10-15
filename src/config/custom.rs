use std::path;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct CustomConfig {
    pub package: Package,
    pub cluster: Cluster,
    #[serde(default, rename = "job")]
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

    #[serde(default = "default_number_of_workers")]
    pub number_of_workers: usize,

    #[serde(default)]
    pub dependencies: Vec<String>,

    #[serde(default)]
    pub pre_setup_commands: super::ProcessState<Vec<String>>,

    #[serde(default)]
    pub post_setup_commands: super::ProcessState<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "provider")]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Aws(AwsCluster),
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct AwsCluster {
    #[serde(default = "default_region")]
    pub region: String,

    #[serde(default = "super::ProcessState::empty")]
    pub ssh_user: super::ProcessState<String>,

    #[serde(default = "super::ProcessState::empty")]
    pub ssh_private_key: super::ProcessState<Option<path::PathBuf>>,

    #[serde(default = "super::ProcessState::empty")]
    pub iam_instance_profile_arn: super::ProcessState<Option<String>>,

    pub template: Option<AwsTemplateType>,
    pub custom: Option<AwsCustom>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AwsTemplateType {
    Light,
    Normal,
    Gpus,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct AwsCustom {
    #[serde(default = "super::ProcessState::empty")]
    pub image_id: super::ProcessState<String>,

    #[serde(default = "super::ProcessState::empty")]
    pub instance_type: super::ProcessState<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Job {
    pub name: String,
    pub working_dir: path::PathBuf,
    pub command: String,
}

fn default_number_of_workers() -> usize {
    2
}

fn default_region() -> String {
    "us-west-2".into()
}
