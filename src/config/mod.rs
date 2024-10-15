pub mod ray;

use std::path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct CustomConfig {
    pub package: Package,
    pub cluster: Cluster,
    #[serde(rename = "job", default)]
    pub jobs: Vec<Job>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Package {
    pub daft_launcher_version: semver::Version,
    pub name: String,
    pub python_version: Option<semver::VersionReq>,
    pub ray_version: Option<semver::VersionReq>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "provider")]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Aws(AwsCluster),
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct AwsCluster {
    pub template: Option<AwsTemplate>,
    #[serde(flatten)]
    pub custom: Option<AwsCustom>,
    #[serde(default = "default_region")]
    pub region: String,
    #[serde(default = "default_ssh_user")]
    pub ssh_user: String,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AwsTemplate {
    #[default]
    Light,
    Normal,
    Gpus,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct AwsCustom {
    #[serde(default = "default_image_id")]
    pub image_id: String,
    #[serde(default = "default_instance_type")]
    pub instance_type: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Job {
    pub name: String,
    pub working_dir: path::PathBuf,
    pub command: String,
}

fn default_region() -> String {
    "us-west-2".to_string()
}

fn default_ssh_user() -> String {
    "ec2-user".to_string()
}

fn default_number_of_workers() -> usize {
    2
}

fn default_image_id() -> String {
    "ami-01c3c55948a949a52".to_string()
}

fn default_instance_type() -> String {
    "m7g.medium".to_string()
}
