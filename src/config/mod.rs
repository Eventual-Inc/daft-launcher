pub mod custom;
pub mod ray;

use std::{
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Context;
use custom::{AwsTemplateType, Cluster};
use semver::{Version, VersionReq};
use tempdir::TempDir;
use which::which;

use crate::config::{
    custom::{CustomConfig, Provider},
    ray::RayConfig,
};

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
        let version_req = String::from_utf8(output.stdout)?;
        let version_req = version_req
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
    let minimum_python_version = "3.9".parse::<VersionReq>().unwrap();
    if minimum_python_version.matches(&version) {
        Ok(format!("={version}").parse().unwrap())
    } else {
        anyhow::bail!("Python version {version} is not supported; must be >= {minimum_python_version}")
    }
}

fn get_ray_version() -> anyhow::Result<VersionReq> {
    let version = get_version("ray", "ray, version ")?;
    Ok(format!("={version}").parse().unwrap())
}

fn get_ssh_private_key() -> anyhow::Result<PathBuf> {
    todo!()
}

fn process(mut custom_config: CustomConfig) -> anyhow::Result<CustomConfig> {
    let provider = match custom_config.cluster.provider {
        Provider::Aws(mut aws_cluster) => {
            let specified_template = aws_cluster.template.is_some();
            let specified_custom = aws_cluster.custom.is_some();
            if specified_template && specified_custom {
                return Err(anyhow::anyhow!(
                    "Cannot specify both template and custom in the AWS cluster configuration"
                ));
            } else if !specified_template && !specified_custom {
                return Err(anyhow::anyhow!(
                    "Must specify either template or custom in the AWS cluster configuration"
                ));
            }

            custom_config.package.python_version = custom_config
                .package
                .python_version
                .map_or_else(get_python_version, Ok)
                .transpose()?;
            custom_config.package.ray_version = custom_config
                .package
                .ray_version
                .map_or_else(get_ray_version, Ok)
                .transpose()?;
            aws_cluster.ssh_user =
                aws_cluster.ssh_user.or_else(|| match aws_cluster.template {
                    Some(AwsTemplateType::Light | AwsTemplateType::Normal)
                    | None => "ec2-user".into(),
                    Some(AwsTemplateType::Gpus) => "ubuntu".into(),
                });
            // aws_cluster.ssh_private_key = aws_cluster
            //     .ssh_private_key
            //     .map_or_else(get_ssh_private_key, Ok)
            //     .transpose()?;
            Provider::Aws(aws_cluster)
        }
    };
    Ok(CustomConfig {
        cluster: Cluster {
            provider,
            ..custom_config.cluster
        },
        ..custom_config
    })
}

pub fn read_custom(path: &Path) -> anyhow::Result<CustomConfig> {
    let mut file =
        OpenOptions::new().read(true).open(path).with_context(|| {
            format!("No configuration file found at the path {path:?}")
        })?;
    let mut buf = String::new();
    let _ = file
        .read_to_string(&mut buf)
        .with_context(|| format!("Failed to read file {path:?}"))?;
    let custom_config = toml::from_str(&buf)?;
    let custom_config = process(custom_config)?;
    Ok(custom_config)
}

pub fn write_ray(ray: RayConfig) -> anyhow::Result<(TempDir, PathBuf)> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::custom::{
            AwsCluster, AwsTemplateType, Cluster, CustomConfig, Job, Package,
            Provider,
        },
        processable_option::ProcessableOption,
    };

    #[test]
    fn test_processing_simple_toml() {
        let custom_config = read_custom(&PathBuf::from(path_from_root!(
            "assets" / "tests" / "light.toml"
        )))
        .unwrap();
        assert_eq!(
            custom_config,
            CustomConfig {
                package: Package {
                    name: "hello-world".into(),
                    daft_launcher_version: "0.4.0-alpha0".parse().unwrap(),
                    python_version: ProcessableOption::Processed(
                        get_python_version().unwrap()
                    ),
                    ray_version: ProcessableOption::Processed(
                        get_ray_version().unwrap()
                    ),
                },
                cluster: Cluster {
                    provider: Provider::Aws(AwsCluster {
                        region: "us-west-2".into(),
                        ssh_user: ProcessableOption::Processed(
                            "ec2-user".into()
                        ),
                        ssh_private_key: ProcessableOption::Raw(None),
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
        );
    }
}
