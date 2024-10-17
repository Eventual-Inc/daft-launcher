use std::path::PathBuf;

use semver::{Version, VersionReq};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct RawConfig {
    pub package: Package,
    pub cluster: Cluster,
    #[serde(default, rename = "job")]
    pub jobs: Vec<Job>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Package {
    pub daft_launcher_version: Version,
    pub name: String,
    pub python_version: Option<VersionReq>,
    pub ray_version: Option<VersionReq>,
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
    pub ssh_user: Option<String>,
    pub ssh_private_key: Option<PathBuf>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwsOverrides {
    pub region: String,
    pub ssh_user: String,
    pub ssh_private_key: PathBuf,
    pub iam_instance_profile_arn: Option<String>,
    pub image_id: String,
    pub instance_type: String,
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

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};

    use super::*;

    #[fixture]
    fn light_toml() -> RawConfig {
        RawConfig {
            package: Package {
                name: "light".into(),
                daft_launcher_version: "0.4.0-alpha0".parse().unwrap(),
                python_version: None,
                ray_version: None,
            },
            cluster: Cluster {
                provider: Provider::Aws(AwsCluster {
                    region: "us-west-2".into(),
                    ssh_user: None,
                    ssh_private_key: None,
                    iam_instance_profile_arn: None,
                    template: Some(AwsTemplateType::Light),
                    custom: None,
                }),
                number_of_workers: 2,
                dependencies: vec![],
                pre_setup_commands: vec![],
                post_setup_commands: vec![],
            },
            jobs: vec![Job {
                name: "filter".into(),
                working_dir: "jobs".into(),
                command: "python filter.py".into(),
            }],
        }
    }

    #[fixture]
    fn custom_toml() -> RawConfig {
        RawConfig {
            package: Package {
                name: "custom".into(),
                daft_launcher_version: "0.1.0".parse().unwrap(),
                python_version: None,
                ray_version: None,
            },
            cluster: Cluster {
                provider: Provider::Aws(AwsCluster {
                    region: "us-east-2".into(),
                    ssh_user: None,
                    ssh_private_key: None,
                    iam_instance_profile_arn: None,
                    template: None,
                    custom: Some(AwsCustom {
                        image_id: Some("...".into()),
                        instance_type: Some("...".into()),
                    }),
                }),
                number_of_workers: 4,
                dependencies: vec![
                    "pytorch".into(),
                    "pandas".into(),
                    "numpy".into(),
                ],
                pre_setup_commands: vec!["echo 'Hello, world!'".into()],
                post_setup_commands: vec!["echo 'Finished!'".into()],
            },
            jobs: vec![
                Job {
                    name: "filter".into(),
                    working_dir: "jobs".into(),
                    command: "python filter.py".into(),
                },
                Job {
                    name: "dedupe".into(),
                    working_dir: "jobs".into(),
                    command: "python dedupe.py".into(),
                },
            ],
        }
    }

    #[rstest]
    #[case(read_toml!("assets" / "tests" / "light.toml"), light_toml())]
    #[case(read_toml!("assets" / "tests" / "custom.toml"), custom_toml())]
    fn test_deser(#[case] actual: RawConfig, #[case] expected: RawConfig) {
        assert_eq!(actual, expected);
    }
}
