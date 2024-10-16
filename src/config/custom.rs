use std::path::PathBuf;

use serde::Deserialize;

use crate::processable_option::ProcessableOption;

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
    pub python_version: ProcessableOption<semver::VersionReq>,
    pub ray_version: ProcessableOption<semver::VersionReq>,
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
    #[serde(default = "default_region")]
    pub region: String,
    pub ssh_user: ProcessableOption<String>,
    pub ssh_private_key: ProcessableOption<PathBuf>,
    pub iam_instance_profile_arn: Option<String>,
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
    pub image_id: Option<String>,
    pub instance_type: Option<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Job {
    pub name: String,
    pub working_dir: PathBuf,
    pub command: String,
}

fn default_number_of_workers() -> usize {
    2
}

fn default_region() -> String {
    "us-west-2".into()
}
