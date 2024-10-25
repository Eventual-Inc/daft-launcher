use std::{
    process::{Command, Stdio},
    sync::LazyLock,
};

use semver::{Version, VersionReq};
use which::which;

use crate::{
    config::{
        defaults::{
            default_region, default_ssh_user, light_image_id,
            light_instance_type, normal_image_id, normal_instance_type,
            DEFAULT_NUMBER_OF_WORKERS,
        },
        raw,
        raw::Job,
        PathRef,
    },
    StrRef,
};

static MIN_PYTHON_VERSION: LazyLock<VersionReq> =
    LazyLock::new(|| "3.9".parse().unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessedConfig {
    pub package: Package,
    pub cluster: Cluster,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub daft_launcher_version: VersionReq,
    pub name: StrRef,
    pub python_version: VersionReq,
    pub ray_version: VersionReq,
}

impl TryFrom<raw::Package> for Package {
    type Error = anyhow::Error;

    fn try_from(value: raw::Package) -> Result<Self, Self::Error> {
        let python_version =
            value.python_version.map_or_else(get_python_version, Ok)?;
        let ray_version = value.ray_version.map_or_else(get_ray_version, Ok)?;
        Ok(Self {
            daft_launcher_version: value.daft_launcher_version,
            name: value.name,
            python_version,
            ray_version,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cluster {
    pub provider: Provider,
    pub number_of_workers: usize,
    pub dependencies: Vec<StrRef>,
    pub pre_setup_commands: Vec<StrRef>,
    pub post_setup_commands: Vec<StrRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    Aws(AwsCluster),
}

impl Provider {
    pub fn process(
        provider: raw::Provider,
    ) -> anyhow::Result<(Self, Vec<StrRef>, Vec<StrRef>)> {
        let (provider, pre_setup_commands, post_setup_commands) = match provider
        {
            raw::Provider::Aws(aws_cluster) => {
                match (aws_cluster.template, aws_cluster.custom) {
                    (Some(..), Some(..)) => anyhow::bail!("Cannot specify both a template type and custom configurations in the configuration file"),
                    (None, None) => anyhow::bail!("Please specify either a template type or some custom configurations in the configuration file"),

                    (None, Some(custom)) => (
                        Provider::Aws(AwsCluster {
                            region: aws_cluster.region.unwrap_or_else(default_region),
                            ssh_user: aws_cluster.ssh_user.unwrap_or_else(default_ssh_user),
                            ssh_private_key: aws_cluster.ssh_private_key,
                            iam_instance_profile_arn: aws_cluster.iam_instance_profile_arn,
                            image_id: custom.image_id,
                            instance_type: custom.instance_type,
                        }),
                        vec![],
                        vec![],
                    ),
                    (Some(raw::AwsTemplateType::Light), None) => (
                        Provider::Aws(AwsCluster {
                            region: aws_cluster.region.unwrap_or_else(default_region),
                            ssh_user: aws_cluster.ssh_user.unwrap_or_else(default_ssh_user),
                            ssh_private_key: aws_cluster.ssh_private_key,
                            iam_instance_profile_arn: aws_cluster.iam_instance_profile_arn,
                            image_id: light_image_id(),
                            instance_type: light_instance_type(),
                        }),
                        vec![],
                        vec![],
                    ),
                    (Some(raw::AwsTemplateType::Normal), None) => (
                        Provider::Aws(AwsCluster {
                            region: aws_cluster.region.unwrap_or_else(default_region),
                            ssh_user: aws_cluster.ssh_user.unwrap_or_else(|| "ec2-user".into()),
                            ssh_private_key: aws_cluster.ssh_private_key,
                            iam_instance_profile_arn: aws_cluster.iam_instance_profile_arn,
                            image_id: normal_image_id(),
                            instance_type: normal_instance_type(),
                        }),
                        vec![],
                        vec![],
                    ),
                    (Some(raw::AwsTemplateType::Gpus), None) => todo!(),
                }
            }
        };
        Ok((provider, pre_setup_commands, post_setup_commands))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwsCluster {
    pub region: StrRef,
    pub ssh_user: StrRef,
    pub ssh_private_key: PathRef,
    pub iam_instance_profile_arn: Option<StrRef>,
    pub image_id: StrRef,
    pub instance_type: StrRef,
}

impl TryFrom<raw::RawConfig> for ProcessedConfig {
    type Error = anyhow::Error;

    fn try_from(mut raw: raw::RawConfig) -> Result<Self, Self::Error> {
        let package = raw.package.try_into()?;
        let (provider, pre_setup_commands, post_setup_commands) =
            Provider::process(raw.cluster.provider)?;
        raw.cluster.pre_setup_commands.extend(pre_setup_commands);
        raw.cluster.post_setup_commands.extend(post_setup_commands);
        Ok(ProcessedConfig {
            package,
            cluster: Cluster {
                provider,
                number_of_workers: raw
                    .cluster
                    .number_of_workers
                    .unwrap_or(DEFAULT_NUMBER_OF_WORKERS),
                dependencies: raw.cluster.dependencies,
                pre_setup_commands: raw.cluster.pre_setup_commands,
                post_setup_commands: raw.cluster.post_setup_commands,
            },
            jobs: raw.jobs,
        })
    }
}

fn get_version(executable: &str, prefix: &str) -> anyhow::Result<Version> {
    if which(executable).is_err() {
        anyhow::bail!("Cannot find a(n) {executable} executable in your $PATH; failed to autodetect {executable} version")
    };
    let output = Command::new(executable)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()?;
    if output.status.success() {
        let version_req = String::from_utf8(output.stdout)?
            .strip_prefix(prefix)
            .unwrap()
            .strip_suffix("\n")
            .unwrap()
            .parse()
            .unwrap();
        Ok(version_req)
    } else {
        anyhow::bail!("Failed to run `{executable} --version`")
    }
}

fn get_python_version() -> anyhow::Result<VersionReq> {
    let version = get_version("python", "Python ")?;
    if MIN_PYTHON_VERSION.matches(&version) {
        log::debug!("Python version determined to be: {version}");
        Ok(format!("={version}").parse().unwrap())
    } else {
        anyhow::bail!("Python version {version} is not supported; must be >= {MIN_PYTHON_VERSION:?}")
    }
}

fn get_ray_version() -> anyhow::Result<VersionReq> {
    let version = get_version("ray", "ray, version ")?;
    log::debug!("Ray version determined to be: {version}");
    Ok(format!("={version}").parse().unwrap())
}

#[cfg(test)]
pub mod tests {
    use rstest::{fixture, rstest};

    use super::*;
    use crate::path_ref;

    #[fixture]
    pub fn light_processed_config() -> ProcessedConfig {
        ProcessedConfig {
            package: Package {
                name: "light".into(),
                daft_launcher_version: "0.4.0-alpha0".parse().unwrap(),
                python_version: get_python_version().unwrap(),
                ray_version: get_ray_version().unwrap(),
            },
            cluster: Cluster {
                provider: Provider::Aws(AwsCluster {
                    region: "us-west-2".into(),
                    ssh_user: "ec2-user".into(),
                    ssh_private_key: path_ref("tests/fixtures/test.pem"),
                    iam_instance_profile_arn: None,
                    image_id: "ami-07c5ecd8498c59db5".into(),
                    instance_type: "t2.nano".into(),
                }),
                number_of_workers: 2,
                dependencies: vec![],
                pre_setup_commands: vec![],
                post_setup_commands: vec![],
            },
            jobs: vec![Job {
                name: "filter".into(),
                working_dir: path_ref("tests"),
                command: "python filter.py".into(),
            }],
        }
    }

    #[fixture]
    pub fn custom_processed_config() -> ProcessedConfig {
        ProcessedConfig {
            package: Package {
                name: "custom".into(),
                daft_launcher_version: "0.1.0".parse().unwrap(),
                python_version: get_python_version().unwrap(),
                ray_version: get_ray_version().unwrap(),
            },
            cluster: Cluster {
                provider: Provider::Aws(AwsCluster {
                    region: "us-east-2".into(),
                    ssh_user: "ec2-user".into(),
                    ssh_private_key: path_ref("tests/fixtures/test.pem"),
                    iam_instance_profile_arn: None,
                    image_id: "...".into(),
                    instance_type: "...".into(),
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
                    working_dir: path_ref("tests"),
                    command: "python filter.py".into(),
                },
                Job {
                    name: "dedupe".into(),
                    working_dir: path_ref("tests"),
                    command: "python dedupe.py".into(),
                },
            ],
        }
    }

    #[rstest]
    #[case(raw::tests::light_raw_config(), light_processed_config())]
    #[case(raw::tests::custom_raw_config(), custom_processed_config())]
    fn test_raw_config_to_processed_config(
        #[case] input: raw::RawConfig,
        #[case] expected: ProcessedConfig,
    ) {
        let actual: ProcessedConfig = input.try_into().unwrap();
        assert_eq!(actual, expected);
    }
}
