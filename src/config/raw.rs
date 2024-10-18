use std::path::Path;

use semver::VersionReq;
use serde::{de::Error, Deserialize, Deserializer};

use crate::{config::PathRef, utils::expand, StrRef};

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawConfig {
    pub package: Package,
    pub cluster: Cluster,
    #[serde(default, rename = "job")]
    pub jobs: Vec<Job>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub daft_launcher_version: VersionReq,
    pub name: StrRef,
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
    pub dependencies: Vec<StrRef>,
    #[serde(default)]
    pub pre_setup_commands: Vec<StrRef>,
    #[serde(default)]
    pub post_setup_commands: Vec<StrRef>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "provider")]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Provider {
    Aws(AwsCluster),
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AwsCluster {
    #[serde(default = "default_region")]
    pub region: StrRef,
    pub ssh_user: Option<StrRef>,
    #[serde(default, deserialize_with = "deserialize_optional_path")]
    pub ssh_private_key: Option<PathRef>,
    pub iam_instance_profile_arn: Option<StrRef>,
    pub template: Option<AwsTemplateType>,
    pub custom: Option<AwsCustomType>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum AwsTemplateType {
    Light,
    Normal,
    Gpus,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AwsCustomType {
    pub image_id: Option<StrRef>,
    pub instance_type: Option<StrRef>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub name: StrRef,
    #[serde(deserialize_with = "deserialize_dir")]
    pub working_dir: PathRef,
    pub command: StrRef,
}

fn default_number_of_workers() -> usize {
    2
}

fn default_region() -> StrRef {
    "us-west-2".into()
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
    expand(path.clone()).map_err(|_| {
        Error::custom(format!(
            "Path expansion failed; could not expand the path '{}'",
            path.display(),
        ))
    })
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
                working_dir: path_ref("assets"),
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
                    region: "us-east-2".into(),
                    ssh_user: None,
                    ssh_private_key: None,
                    iam_instance_profile_arn: None,
                    template: None,
                    custom: Some(AwsCustomType {
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
                    working_dir: path_ref("assets"),
                    command: "python filter.py".into(),
                },
                Job {
                    name: "dedupe".into(),
                    working_dir: path_ref("assets"),
                    command: "python dedupe.py".into(),
                },
            ],
        }
    }

    #[rstest]
    #[case(include_str!(path_from_root!("assets" / "tests" / "light.toml")), light_raw_config())]
    #[case(include_str!(path_from_root!("assets" / "tests" / "custom.toml")), custom_raw_config())]
    fn test_str_to_raw_config(
        #[case] input: &str,
        #[case] expected: RawConfig,
    ) {
        let actual: RawConfig = toml::from_str(input).unwrap();
        assert_eq!(actual, expected);
    }
}
