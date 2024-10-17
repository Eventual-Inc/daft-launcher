use std::path::{Path, PathBuf};

use semver::{Version, VersionReq};
use serde::{de::Error, Deserialize, Deserializer};

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
    #[serde(default, deserialize_with = "deserialize_optional_path")]
    pub ssh_private_key: Option<PathBuf>,
    pub iam_instance_profile_arn: Option<String>,
    pub template: Option<AwsTemplateType>,
    pub custom: Option<AwsCustomType>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AwsTemplateType {
    Light,
    Normal,
    Gpus,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct AwsCustomType {
    pub image_id: Option<String>,
    pub instance_type: Option<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Job {
    pub name: String,
    #[serde(deserialize_with = "deserialize_dir")]
    pub working_dir: PathBuf,
    pub command: String,
}

fn default_number_of_workers() -> usize {
    2
}

fn default_region() -> String {
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

fn deserialize_dir<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let path = PathBuf::deserialize(deserializer)?;
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
) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    let path = match Option::<PathBuf>::deserialize(deserializer).expect("asdf")
    {
        Some(path) => path,
        None => return Ok(None),
    };
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
                working_dir: "assets".into(),
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
                    working_dir: "assets".into(),
                    command: "python filter.py".into(),
                },
                Job {
                    name: "dedupe".into(),
                    working_dir: "assets".into(),
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
