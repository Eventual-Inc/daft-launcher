use std::path;

use serde::Serialize;

use crate::{config::custom, utils};

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct RayConfig {
    pub cluster_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_workers: Option<usize>,
    pub provider: Provider,
    pub auth: Auth,
    pub available_node_types: hashbrown::HashMap<String, NodeType>,
    pub initialization_commands: Vec<String>,
    pub setup_commands: Vec<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Provider {
    pub r#type: String,
    pub region: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Auth {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_private_key: Option<path::PathBuf>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct NodeType {
    pub node_config: NodeConfig,
    pub min_workers: usize,
    pub max_workers: usize,
    pub resources: Resources,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum NodeConfig {
    #[serde(untagged)]
    Aws(AwsNodeConfig),
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct AwsNodeConfig {
    #[serde(rename = "InstanceType")]
    pub instance_type: String,

    #[serde(
        rename = "IamInstanceProfile",
        skip_serializing_if = "Option::is_none"
    )]
    pub iam_instance_profile: Option<IamInstanceProfile>,

    #[serde(rename = "ImageId")]
    pub image_id: String,

    #[serde(rename = "KeyName")]
    pub key_name: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct IamInstanceProfile {
    #[serde(rename = "Arn")]
    pub arn: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Resources {
    #[serde(rename = "CPU")]
    pub cpu: usize,
    #[serde(rename = "GPU")]
    pub gpu: usize,
}

impl TryFrom<custom::CustomConfig> for RayConfig {
    type Error = anyhow::Error;

    fn try_from(custom_config: custom::CustomConfig) -> anyhow::Result<Self> {
        utils::custom_to_ray_config(custom_config)
    }
}
