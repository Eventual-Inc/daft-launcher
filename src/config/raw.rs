use std::{path::Path, str::FromStr};

use semver::VersionReq;
use serde::{de::Error, Deserialize, Deserializer, Serialize};

use crate::{
    config::{PathRef, Selectable},
    path_ref,
    utils::expand,
    StrRef,
};

#[derive(Default, Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawConfig {
    pub package: Package,
    pub cluster: Cluster,
    #[serde(default, rename = "job", skip_serializing_if = "Vec::is_empty")]
    pub jobs: Vec<Job>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub daft_launcher_version: VersionReq,
    pub name: StrRef,
    pub python_version: Option<VersionReq>,
    pub ray_version: Option<VersionReq>,
}

pub fn default_name() -> String {
    "my-cluster".into()
}

impl Default for Package {
    fn default() -> Self {
        Self {
            daft_launcher_version: env!("CARGO_PKG_VERSION").parse().unwrap(),
            name: default_name().into(),
            python_version: None,
            ray_version: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Cluster {
    #[serde(flatten)]
    pub provider: Provider,
    #[serde(default)]
    pub number_of_workers: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<StrRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pre_setup_commands: Vec<StrRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_setup_commands: Vec<StrRef>,
}

impl Default for Cluster {
    fn default() -> Self {
        Self {
            number_of_workers: None,
            provider: Provider::default(),
            dependencies: vec![],
            pre_setup_commands: vec![],
            post_setup_commands: vec![],
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "provider")]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Provider {
    Aws(AwsCluster),
}

impl Default for Provider {
    fn default() -> Self {
        Self::Aws(AwsCluster::default())
    }
}

impl Selectable for Provider {
    fn to_options() -> &'static [&'static str] {
        &["aws"]
    }
}

impl FromStr for Provider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        match s {
            "aws" => Ok(Self::Aws(AwsCluster::default())),
            _ => Err(anyhow::anyhow!("Unknown provider: {}", s)),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AwsCluster {
    #[serde(default)]
    pub region: Option<StrRef>,
    pub ssh_user: Option<StrRef>,
    #[serde(default, deserialize_with = "deserialize_optional_path")]
    pub ssh_private_key: Option<PathRef>,
    pub iam_instance_profile_arn: Option<StrRef>,
    pub template: Option<AwsTemplateType>,
    pub custom: Option<AwsCustomType>,
}

impl Default for AwsCluster {
    fn default() -> Self {
        Self {
            region: None,
            ssh_user: None,
            ssh_private_key: None,
            iam_instance_profile_arn: None,
            template: Some(AwsTemplateType::default()),
            custom: None,
        }
    }
}

#[derive(
    Default, Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum AwsTemplateType {
    #[default]
    Light,
    Normal,
    Gpus,
}

impl Selectable for AwsTemplateType {
    fn to_options() -> &'static [&'static str] {
        &["light", "normal", "gpus"]
    }
}

impl FromStr for AwsTemplateType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "light" => Ok(Self::Light),
            "normal" => Ok(Self::Normal),
            "gpus" => Ok(Self::Gpus),
            _ => Err(anyhow::anyhow!("Unknown AWS template type: {}", s)),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AwsCustomType {
    pub image_id: Option<StrRef>,
    pub instance_type: Option<StrRef>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub name: StrRef,
    #[serde(deserialize_with = "deserialize_dir")]
    pub working_dir: PathRef,
    pub command: StrRef,
}

fn assert_path_exists<'de, D>(path: &Path) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    if path.exists() {
        Ok(())
    } else {
        Err(Error::custom(format!(
            "The path '{}' does not exist.",
            path.display(),
        )))
    }
}

fn expand_helper<'de, D>(path: PathRef) -> Result<PathRef, D::Error>
where
    D: Deserializer<'de>,
{
    let path = expand(&path).map_err(|_| {
        Error::custom(format!(
            "Path expansion failed; could not expand the path '{}'",
            path.display(),
        ))
    })?;
    Ok(path_ref(path))
}

fn deserialize_dir<'de, D>(deserializer: D) -> Result<PathRef, D::Error>
where
    D: Deserializer<'de>,
{
    let path = PathRef::deserialize(deserializer)?;
    let path = expand_helper::<D>(path)?;
    assert_path_exists::<D>(&path)?;
    if !path.is_dir() {
        return Err(Error::custom(format!(
            "The path '{}' is not a directory.",
            path.display(),
        )));
    }
    Ok(path)
}

fn deserialize_optional_path<'de, D>(
    deserializer: D,
) -> Result<Option<PathRef>, D::Error>
where
    D: Deserializer<'de>,
{
    let path = match Option::deserialize(deserializer)? {
        Some(path) => path,
        None => return Ok(None),
    };
    let path = expand_helper::<D>(path)?;
    assert_path_exists::<D>(&path)?;
    if !path.is_file() {
        return Err(Error::custom(format!(
            "The path '{}' is not a file.",
            path.display(),
        )));
    }
    Ok(Some(path))
}

#[cfg(test)]
pub mod tests {
    use rstest::{fixture, rstest};

    use super::*;
    use crate::path_ref;

    #[fixture]
    pub fn light_raw_config() -> RawConfig {
        RawConfig {
            package: Package {
                name: "light".into(),
                daft_launcher_version: "0.4.0-alpha0".parse().unwrap(),
                python_version: None,
                ray_version: None,
            },
            cluster: Cluster {
                provider: Provider::Aws(AwsCluster {
                    region: None,
                    ssh_user: None,
                    ssh_private_key: None,
                    iam_instance_profile_arn: None,
                    template: Some(AwsTemplateType::Light),
                    custom: None,
                }),
                number_of_workers: None,
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
    pub fn custom_raw_config() -> RawConfig {
        RawConfig {
            package: Package {
                name: "custom".into(),
                daft_launcher_version: "0.1.0".parse().unwrap(),
                python_version: None,
                ray_version: None,
            },
            cluster: Cluster {
                provider: Provider::Aws(AwsCluster {
                    region: Some("us-east-2".into()),
                    ssh_user: None,
                    ssh_private_key: None,
                    iam_instance_profile_arn: None,
                    template: None,
                    custom: Some(AwsCustomType {
                        image_id: Some("...".into()),
                        instance_type: Some("...".into()),
                    }),
                }),
                number_of_workers: Some(4),
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
    #[case(include_str!(path_from_root!("tests" / "fixtures" / "light.toml")), light_raw_config())]
    #[case(include_str!(path_from_root!("tests" / "fixtures" / "custom.toml")), custom_raw_config())]
    fn test_str_to_raw_config(
        #[case] input: &str,
        #[case] expected: RawConfig,
    ) {
        let actual: RawConfig = toml::from_str(input).unwrap();
        assert_eq!(actual, expected);
    }
}
