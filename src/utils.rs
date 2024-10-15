use std::{
    fs,
    io::{self, Read, Write},
    path::{self, PathBuf},
};

use anyhow::Context;
use map_macro::hashbrown as hb;

use crate::config::{self, ray};

pub async fn is_authenticated_with_aws() -> bool {
    // let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
    //     .region(aws_config::meta::region::RegionProviderChain::default_provider())
    //     .load()
    //     .await;
    // let client = aws_sdk_sts::Client::new(&sdk_config);
    // client.get_caller_identity().send().await.is_ok()
    true
}

pub async fn assert_is_authenticated_with_aws() -> anyhow::Result<()> {
    if is_authenticated_with_aws().await {
        Ok(())
    } else {
        anyhow::bail!("You are not signed in to AWS; please sign in first")
    }
}

pub fn read_custom_config(path: &path::Path) -> anyhow::Result<config::CustomConfig> {
    let mut file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .with_context(|| format!("No configuration file found at the path {path:?}"))?;
    let mut buf = String::new();
    let _ = file
        .read_to_string(&mut buf)
        .with_context(|| format!("Failed to read file {path:?}"))?;
    let custom_config = toml::from_str(&buf)?;
    Ok(custom_config)
}

pub fn write_ray_config(
    ray_config: &ray::RayConfig,
) -> anyhow::Result<(tempdir::TempDir, path::PathBuf)> {
    let temp_dir = tempdir::TempDir::new("ray_config")
        .expect("Creation of temporary directory should always succeed");
    let path = temp_dir.path().join("ray.yaml");
    let ray_config =
        serde_yaml::to_string(ray_config).expect("Serialization should always succeed");
    create_new_file(&path)
        .expect("Creating new file in temporary directory should always succeed")
        .write_all(ray_config.as_bytes())
        .expect("Writing to file should always succeed");
    Ok((temp_dir, path))
}

pub fn create_new_file(path: &path::Path) -> anyhow::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => anyhow::anyhow!("The file {:?} already exists", path),
            _ => error.into(),
        })
}

pub struct AwsOverridden {
    pub image_id: String,
    pub instance_type: String,
    pub region: String,
    pub ssh_user: String,
    pub ssh_private_key: Option<PathBuf>,
    pub iam_instance_profile_arn: Option<String>,
    pub pre_setup_commands: Vec<String>,
    pub post_setup_commands: Vec<String>,
}

fn aws_template_light(
    pre_setup_commands: Vec<String>,
    post_setup_commands: Vec<String>,
    aws_cluster: config::AwsCluster,
) -> AwsOverridden {
    AwsOverridden {
        image_id: "ami-07c5ecd8498c59db5".into(),
        instance_type: "t2.nano".into(),
        region: aws_cluster.region,
        ssh_user: aws_cluster.ssh_user.unwrap_or_else(ssh_ec2_user),
        ssh_private_key: aws_cluster.ssh_private_key,
        iam_instance_profile_arn: aws_cluster.iam_instance_profile_arn,
        pre_setup_commands,
        post_setup_commands,
    }
}

fn aws_template_normal(
    pre_setup_commands: Vec<String>,
    post_setup_commands: Vec<String>,
    aws_cluster: config::AwsCluster,
) -> AwsOverridden {
    AwsOverridden {
        image_id: "ami-07dcfc8123b5479a8".into(),
        instance_type: "m7g.medium".into(),
        region: aws_cluster.region,
        ssh_user: aws_cluster.ssh_user.unwrap_or_else(ssh_ec2_user),
        ssh_private_key: aws_cluster.ssh_private_key,
        iam_instance_profile_arn: aws_cluster.iam_instance_profile_arn,
        pre_setup_commands,
        post_setup_commands,
    }
}

fn aws_template_gpus(
    pre_setup_commands: Vec<String>,
    post_setup_commands: Vec<String>,
    aws_cluster: config::AwsCluster,
) -> AwsOverridden {
    todo!()
}

fn ssh_ec2_user() -> String {
    "ec2-user".into()
}

fn ssh_ubuntu_user() -> String {
    "ubuntu".into()
}

fn image_id() -> String {
    "ami-01c3c55948a949a52".to_string()
}

fn instance_type() -> String {
    "m7g.medium".to_string()
}

pub fn default_region() -> String {
    "us-west-2".to_string()
}

pub fn default_number_of_workers() -> usize {
    2
}

fn override_aws(
    pre_setup_commands: Vec<String>,
    post_setup_commands: Vec<String>,
    mut aws_cluster: config::AwsCluster,
) -> anyhow::Result<AwsOverridden> {
    let overridden = match (aws_cluster.template.as_mut(), aws_cluster.custom.as_mut()) {
        (Some(..), Some(..)) => anyhow::bail!("Both template and custom cluster configurations are specified; please specify only one or the other"),

        (Some(config::AwsTemplate::Light), None) => aws_template_light(pre_setup_commands, post_setup_commands, aws_cluster),
        (Some(config::AwsTemplate::Normal), None) => aws_template_normal(pre_setup_commands, post_setup_commands, aws_cluster),
        (Some(config::AwsTemplate::Gpus), None) => aws_template_gpus(pre_setup_commands, post_setup_commands, aws_cluster),

        (None, Some(overridable)) => {
            AwsOverridden {
                image_id: overridable.image_id.take().unwrap_or_else(image_id),
                instance_type: overridable.instance_type.take().unwrap_or_else(instance_type),
                region: aws_cluster.region,
                ssh_user: aws_cluster.ssh_user.unwrap_or_else(ssh_ec2_user),
                ssh_private_key: aws_cluster.ssh_private_key,
                iam_instance_profile_arn: aws_cluster.iam_instance_profile_arn,
                pre_setup_commands,
                post_setup_commands,
            }
        },

        (None, None) => anyhow::bail!("Neither template nor custom cluster configurations are specified; please specify exactly one of the two"),
    };
    Ok(overridden)
}

pub fn custom_to_ray_config(custom_config: config::CustomConfig) -> anyhow::Result<ray::RayConfig> {
    let ray_config = match custom_config.cluster.provider {
        config::Provider::Aws(aws_cluster) => {
            let overridden = override_aws(
                custom_config.cluster.pre_setup_commands,
                custom_config.cluster.post_setup_commands,
                aws_cluster,
            )?;
            ray::RayConfig {
                cluster_name: custom_config.package.name,
                max_workers: Some(custom_config.cluster.number_of_workers),
                provider: ray::Provider {
                    r#type: "aws".to_string(),
                    region: overridden.region,
                },
                auth: ray::Auth {
                    ssh_user: Some(overridden.ssh_user),
                    ssh_private_key: overridden.ssh_private_key,
                },
                available_node_types: hb::hash_map! {
                    "ray.head.default".to_string() => ray::NodeType {
                        node_config: ray::NodeConfig::Aws(ray::AwsNodeConfig {
                            instance_type: overridden.instance_type,
                            iam_instance_profile: ray::IamInstanceProfile {
                                arn: overridden.iam_instance_profile_arn,
                            },
                            image_id: overridden.image_id,
                        }),
                        min_workers: custom_config.cluster.number_of_workers,
                        max_workers: custom_config.cluster.number_of_workers,
                        resources: ray::Resources { cpu: 1, gpu: 0 },
                    },
                },
                initialization_commands: overridden.pre_setup_commands,
                setup_commands: overridden.post_setup_commands,
            }
        }
    };
    Ok(ray_config)
}
