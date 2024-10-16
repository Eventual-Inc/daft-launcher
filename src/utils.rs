use std::{
    fs, io,
    path::{self, PathBuf},
};

use crate::config::{custom, ray};

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

pub fn create_new_file(path: &path::Path) -> anyhow::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => {
                anyhow::anyhow!("The file {:?} already exists", path)
            }
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

impl AwsOverridden {
    fn new(
        aws_cluster: custom::AwsCluster,
        image_id: String,
        instance_type: String,
        pre_setup_commands: Vec<String>,
        post_setup_commands: Vec<String>,
        f: fn() -> String,
    ) -> Self {
        todo!()
        // Self {
        //     image_id,
        //     instance_type,
        //     region: aws_cluster.region,
        //     ssh_user: aws_cluster.ssh_user.unwrap_or_else(f),
        //     ssh_private_key: aws_cluster.ssh_private_key,
        //     iam_instance_profile_arn: aws_cluster.iam_instance_profile_arn,
        //     pre_setup_commands,
        //     post_setup_commands,
        // }
    }
}

fn aws_template_light_overrides(
    pre_setup_commands: Vec<String>,
    post_setup_commands: Vec<String>,
    aws_cluster: custom::AwsCluster,
) -> AwsOverridden {
    // AwsOverridden::new(
    //     aws_cluster,
    //     "ami-07c5ecd8498c59db5".into(),
    //     "t2.nano".into(),
    //     pre_setup_commands,
    //     post_setup_commands,
    //     ssh_ec2_user,
    // )
    todo!()
}

fn aws_template_normal_overrides(
    pre_setup_commands: Vec<String>,
    post_setup_commands: Vec<String>,
    aws_cluster: custom::AwsCluster,
) -> AwsOverridden {
    // AwsOverridden::new(
    //     aws_cluster,
    //     "ami-07dcfc8123b5479a8".into(),
    //     "m7g.medium".into(),
    //     pre_setup_commands,
    //     post_setup_commands,
    //     ssh_ec2_user,
    // )
    todo!()
}

fn aws_template_gpus_overrides(
    pre_setup_commands: Vec<String>,
    post_setup_commands: Vec<String>,
    aws_cluster: custom::AwsCluster,
) -> AwsOverridden {
    todo!()
}

fn ssh_ec2_user() -> String {
    "ec2-user".into()
}

fn ssh_ubuntu_user() -> String {
    "ubuntu".into()
}

pub fn default_image_id() -> String {
    "ami-01c3c55948a949a52".to_string()
}

pub fn default_instance_type() -> String {
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
    aws_cluster: custom::AwsCluster,
) -> anyhow::Result<AwsOverridden> {
    todo!()
    // let overridden = match (aws_cluster.template, aws_cluster.custom.as_ref()) {
    //     (Some(..), Some(..)) => anyhow::bail!("Both template and custom cluster configurations are specified; please specify only one or the other"),
    //     (Some(custom::AwsTemplateType::Light), None) =>
    //         aws_template_light_overrides(pre_setup_commands, post_setup_commands, aws_cluster),
    //     (Some(custom::AwsTemplateType::Normal), None) =>
    //         aws_template_normal_overrides(pre_setup_commands, post_setup_commands, aws_cluster),
    //     (Some(custom::AwsTemplateType::Gpus), None) =>
    //         aws_template_gpus_overrides(pre_setup_commands, post_setup_commands, aws_cluster),
    //     (None, Some(custom)) => {
    //         let custom = custom.clone();
    //         AwsOverridden::new(aws_cluster, custom.image_id, custom.instance_type, pre_setup_commands , post_setup_commands, ssh_ec2_user)
    //     },
    //     (None, None) => anyhow::bail!("Neither template nor custom cluster configurations are specified; please specify exactly one of the two"),
    // };
    // Ok(overridden)
}

pub fn custom_to_ray_config(
    custom_config: custom::CustomConfig,
) -> anyhow::Result<ray::RayConfig> {
    // let ray_config = match custom_config.cluster.provider {
    //     custom::Provider::Aws(aws_cluster) => {
    //         let overridden = override_aws(
    //             custom_config.cluster.pre_setup_commands,
    //             custom_config.cluster.post_setup_commands,
    //             aws_cluster,
    //         )?;
    //         ray::RayConfig {
    //             cluster_name: custom_config.package.name,
    //             max_workers: Some(custom_config.cluster.number_of_workers),
    //             provider: ray::Provider {
    //                 r#type: "aws".to_string(),
    //                 region: overridden.region,
    //             },
    //             auth: ray::Auth {
    //                 ssh_user: Some(overridden.ssh_user),
    //                 ssh_private_key: overridden.ssh_private_key.clone(),
    //             },
    //             available_node_types: hb::hash_map! {
    //                 "ray.head.default".to_string() => ray::NodeType {
    //                     node_config: ray::NodeConfig::Aws(ray::AwsNodeConfig {
    //                         instance_type: overridden.instance_type,
    //                         iam_instance_profile: overridden.iam_instance_profile_arn.map(|arn| ray::IamInstanceProfile { arn }),
    //                         image_id: overridden.image_id,
    //                         key_name: overridden.ssh_private_key.map(|ssh_private_key| ssh_private_key.file_name().unwrap().to_str().unwrap().to_string()),
    //                     }),
    //                     min_workers: custom_config.cluster.number_of_workers,
    //                     max_workers: custom_config.cluster.number_of_workers,
    //                     resources: ray::Resources { cpu: 1, gpu: 0 },
    //                 },
    //             },
    //             initialization_commands: overridden.pre_setup_commands,
    //             setup_commands: overridden.post_setup_commands,
    //         }
    //     }
    // };
    // Ok(ray_config)
    todo!()
}
