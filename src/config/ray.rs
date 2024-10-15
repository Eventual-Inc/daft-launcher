use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::config;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct RayConfig {
    pub cluster_name: String,
    pub max_workers: Option<usize>,
    pub provider: Provider,
    pub auth: Auth,
    // pub available_node_types: HashMap<String, NodeType>,
    pub initialization_commands: Vec<String>,
    pub setup_commands: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Provider {
    pub r#type: String,
    pub region: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Auth {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_private_key: Option<String>,
}

// #[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
// pub struct NodeType {
//     pub node_config: NodeConfig,
//     pub min_workers: usize,
//     pub max_workers: usize,
//     pub resources: Resources,
// }

// #[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
// pub enum NodeConfig {
//     Aws(AwsNodeConfig),
// }

// #[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
// pub struct AwsNodeConfig {
//     #[serde(rename = "InstanceType")]
//     instance_type: String,

//     #[serde(rename = "InstanceType")]
//     iam_instance_profile: IamInstanceProfile,

//     #[serde(rename = "ImageId")]
//     image_id: String,
// }

// #[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
// pub struct IamInstanceProfile {
//     #[serde(rename = "Arn")]
//     arn: String,
// }

// #[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
// pub struct Resources {
//     #[serde(rename = "CPU")]
//     cpu: usize,
//     #[serde(rename = "GPU")]
//     gpu: usize,
// }

impl From<config::CustomConfig> for RayConfig {
    fn from(custom_config: config::CustomConfig) -> Self {
        match custom_config.cluster.provider {
            config::Provider::Aws(aws_cluster) => Self {
                cluster_name: custom_config.package.name,
                max_workers: Some(custom_config.cluster.number_of_workers),
                provider: Provider {
                    r#type: "aws".to_string(),
                    region: aws_cluster.region,
                },
                auth: Auth {
                    ssh_user: Some(aws_cluster.ssh_user),
                    ssh_private_key: None,
                },
                // available_node_types: HashMap::new(),
                initialization_commands: vec![],
                setup_commands: vec![],
            },
        }
    }
}
