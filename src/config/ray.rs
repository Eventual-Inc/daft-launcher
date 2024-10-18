use std::path::{Path, PathBuf};

use hashbrown::HashMap;
use map_macro::hashbrown::hash_map;
use serde::Serialize;

use crate::{
    config::{processed, StrRef},
    utils::path_to_str,
};

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct RayConfig {
    pub cluster_name: StrRef,
    pub max_workers: usize,
    pub provider: Provider,
    pub auth: Auth,
    pub available_node_types: HashMap<StrRef, NodeType>,
    pub initialization_commands: Vec<StrRef>,
    pub setup_commands: Vec<StrRef>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Provider {
    pub r#type: StrRef,
    pub region: StrRef,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Auth {
    pub ssh_user: StrRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_private_key: Option<PathBuf>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct NodeType {
    pub node_config: NodeConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_workers: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_workers: Option<usize>,
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
    pub instance_type: StrRef,
    #[serde(
        rename = "IamInstanceProfile",
        skip_serializing_if = "Option::is_none"
    )]
    pub iam_instance_profile: Option<IamInstanceProfile>,
    #[serde(rename = "ImageId")]
    pub image_id: StrRef,
    #[serde(rename = "KeyName")]
    pub key_name: Option<StrRef>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct IamInstanceProfile {
    #[serde(rename = "Arn")]
    pub arn: StrRef,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Resources {
    #[serde(rename = "CPU")]
    pub cpu: usize,
    #[serde(rename = "GPU")]
    pub gpu: usize,
}

fn to_key_name(path: &Path) -> anyhow::Result<&str> {
    let path = path.file_name().expect(
        "File should exist, as checked by deserialzation logic in raw.rs",
    );
    let key_name = path_to_str(path)?;
    Ok(key_name)
}

impl TryFrom<processed::ProcessedConfig> for RayConfig {
    type Error = anyhow::Error;

    fn try_from(
        config: processed::ProcessedConfig,
    ) -> Result<Self, Self::Error> {
        let (provider, available_node_types, auth) =
            match config.cluster.provider {
                processed::Provider::Aws(aws_cluster) => (
                    Provider {
                        r#type: "aws".into(),
                        region: aws_cluster.region,
                    },
                    {
                        let generic_node_type = NodeType {
                            node_config: NodeConfig::Aws(AwsNodeConfig {
                                instance_type: aws_cluster.instance_type,
                                iam_instance_profile: aws_cluster
                                    .iam_instance_profile_arn
                                    .map(|arn| IamInstanceProfile { arn }),
                                image_id: aws_cluster.image_id,
                                key_name: aws_cluster
                                    .ssh_private_key
                                    .as_deref()
                                    .map(to_key_name)
                                    .transpose()?
                                    .map(Into::into),
                            }),
                            min_workers: Some(config.cluster.number_of_workers),
                            max_workers: Some(config.cluster.number_of_workers),
                            resources: Resources { cpu: 1, gpu: 0 },
                        };
                        hash_map! {
                            "ray.head.default".into() => NodeType {
                                min_workers: None,
                                max_workers: None,
                                ..generic_node_type.clone()
                            },
                            "ray.worker.default".into() => generic_node_type,
                        }
                    },
                    Auth {
                        ssh_user: aws_cluster.ssh_user,
                        ssh_private_key: aws_cluster.ssh_private_key,
                    },
                ),
            };
        Ok(Self {
            cluster_name: config.package.name,
            max_workers: config.cluster.number_of_workers,
            provider,
            auth,
            available_node_types,
            initialization_commands: vec![],
            setup_commands: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use map_macro::hashbrown::hash_map;
    use rstest::{fixture, rstest};

    use super::*;

    #[fixture]
    pub fn light_ray_config() -> RayConfig {
        RayConfig {
            cluster_name: "light".into(),
            max_workers: 2,
            provider: Provider {
                r#type: "aws".into(),
                region: "us-west-2".into(),
            },
            auth: Auth {
                ssh_user: "ec2-user".into(),
                ssh_private_key: None,
            },
            available_node_types: {
                let generic_node_type = NodeType {
                    node_config: NodeConfig::Aws(AwsNodeConfig {
                        instance_type: "t2.nano".into(),
                        iam_instance_profile: None,
                        image_id: "ami-07c5ecd8498c59db5".into(),
                        key_name: None,
                    }),
                    min_workers: Some(2),
                    max_workers: Some(2),
                    resources: Resources { cpu: 1, gpu: 0 },
                };
                hash_map! {
                    "ray.head.default".into() => NodeType { min_workers: None, max_workers: None, ..generic_node_type.clone() },
                    "ray.worker.default".into() => generic_node_type,
                }
            },
            initialization_commands: vec![],
            setup_commands: vec![],
        }
    }

    #[rstest]
    #[case(processed::tests::light_processed_config(), light_ray_config())]
    fn test_processed_config_to_ray_config(
        #[case] input: processed::ProcessedConfig,
        #[case] expected: RayConfig,
    ) {
        let actual: RayConfig = input.try_into().unwrap();
        assert_eq!(actual, expected);
    }
}
