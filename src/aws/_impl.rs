//! All internal implementations of interacting with the AWS SDK (EC2, STS,
//! etc.).
//!
//! # Note
//! None of these functions should be called directly; only the public APIs
//! exposed in the [`super`] module should call them. These functions should
//! also *NOT* use any widgets defined in [`crate::widgets`]. Those are very
//! user-facing constructs, whereas this module is only concerned with the pure
//! business logic.

use std::{borrow::Cow, net::Ipv4Addr};

use aws_config::{BehaviorVersion, Region};
use aws_sdk_ec2::{types::InstanceStateName, Client};
use console::style;

use crate::StrRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwsInstance {
    pub regular_name: StrRef,
    pub ray_name: StrRef,
    pub key_pair_name: Option<StrRef>,
    pub public_ipv4_address: Option<Ipv4Addr>,
    pub state: Option<InstanceStateName>,
}

impl AwsInstance {
    pub fn name_equals_ray_name(&self, name: impl AsRef<str>) -> bool {
        let name = name.as_ref();
        let name = format!("ray-{}-head", name);
        &*name == &*self.ray_name
    }
}

async fn is_authenticated() -> bool {
    let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::meta::region::RegionProviderChain::default_provider())
        .load()
        .await;
    let client = aws_sdk_sts::Client::new(&sdk_config);
    client.get_caller_identity().send().await.is_ok()
}

pub async fn assert_authenticated() -> anyhow::Result<()> {
    if !is_authenticated().await {
        anyhow::bail!("You are not signed in to AWS; please sign in first");
    };
    Ok(())
}

pub async fn list_instances(region: Cow<'static, str>) -> anyhow::Result<Vec<AwsInstance>> {
    let region = Region::new(region);
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .load()
        .await;
    let client = Client::new(&sdk_config);
    let instances = client.describe_instances().send().await?;
    let reservations = instances.reservations.unwrap_or_default();
    let instance_states = reservations
        .iter()
        .filter_map(|reservation| reservation.instances.as_ref())
        .flatten()
        .filter_map(|instance| {
            instance.tags.as_ref().map(|tags| {
                (
                    instance,
                    tags.iter().filter_map(|tag| tag.key().zip(tag.value())),
                )
            })
        })
        .filter_map(|(instance, tags)| {
            let mut ray_name = None;
            let mut regular_name = None;
            for (key, value) in tags {
                if key == "Name" {
                    ray_name = Some(value.into());
                } else if key == "ray-cluster-name" {
                    regular_name = Some(value.into());
                }
            }
            let ray_name = ray_name?;
            let regular_name = regular_name?;
            Some(AwsInstance {
                regular_name,
                ray_name,
                key_pair_name: instance.key_name().map(Into::into),
                public_ipv4_address: instance
                    .public_ip_address()
                    .and_then(|ip_addr| ip_addr.parse().ok()),
                state: instance
                    .state()
                    .and_then(|instance_state| instance_state.name())
                    .cloned(),
            })
        })
        .collect();
    Ok(instance_states)
}

pub async fn assert_non_clashing_cluster_name(
    name: &str,
    region: Cow<'static, str>,
) -> anyhow::Result<()> {
    fn format_aws_instance_names(name: &str, instances: &[AwsInstance]) -> String {
        let mut joined_names = String::new();
        for (index, instance) in instances.iter().enumerate() {
            if index != 0 {
                joined_names.push_str(&style(", ").green().to_string());
            }
            let styled_name = if instance.name_equals_ray_name(name) {
                style(&instance.ray_name).bold()
            } else {
                style(&instance.ray_name)
            }
            .green()
            .to_string();
            joined_names.push_str(&styled_name);
        }
        joined_names
    }

    let instances = list_instances(region).await?;
    let mut instance_name_already_exists = false;
    for instance in &instances {
        if instance.name_equals_ray_name(name) {
            instance_name_already_exists = true;
        }
    }
    if instance_name_already_exists {
        let names = format_aws_instance_names(&name, &instances);
        anyhow::bail!(r#"An instance with the name "{}" already exists in that specified region; please choose a different name
Instance names: {}
{}"#,
            name,
            names,
            style("*Note that Ray prepends `ray-` before and appends `-head` after the name of your cluster").red(),
        );
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Depends upon global state of AWS authentication; if you want to run this test, run it manually after you've authenticated with AWS"]
    async fn test_is_authenticated_with_aws() {
        assert!(is_authenticated().await);
    }
}
