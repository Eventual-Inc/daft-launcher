use std::{borrow::Cow, net::Ipv4Addr};

use aws_config::{BehaviorVersion, Region};
use aws_sdk_ec2::{types::InstanceStateName, Client};

use crate::{widgets::Spinner, StrRef};

async fn is_authenticated_with_aws() -> bool {
    let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::meta::region::RegionProviderChain::default_provider())
        .load()
        .await;
    let client = aws_sdk_sts::Client::new(&sdk_config);
    client.get_caller_identity().send().await.is_ok()
}

pub async fn assert_is_authenticated_with_aws() -> anyhow::Result<()> {
    let spinner = Spinner::new("Authenticating with AWS");
    let is_authenticated = is_authenticated_with_aws().await;
    if is_authenticated {
        spinner.success();
        Ok(())
    } else {
        anyhow::bail!("You are not signed in to AWS; please sign in first")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwsInstance {
    pub ray_name: StrRef,
    pub key_pair_name: Option<StrRef>,
    pub public_ipv4_address: Option<Ipv4Addr>,
    pub state: Option<InstanceStateName>,
}

impl AwsInstance {
    pub fn name_equals_ray_name(&self, name: &str) -> bool {
        let name = format!("ray-{}-head", name);
        &*name == &*self.ray_name
    }
}

pub async fn list_instances(
    region: impl Into<Cow<'static, str>>,
) -> anyhow::Result<Vec<AwsInstance>> {
    let region: Cow<_> = region.into();
    let spinner = Spinner::new(format!("Listing all AWS instances in region {}", region));
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
            for (key, value) in tags {
                if key == "Name" {
                    ray_name = Some(value.into());
                }
            }
            let ray_name = ray_name?;
            Some(AwsInstance {
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
    spinner.success();
    Ok(instance_states)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Depends upon global state of AWS authentication; if you want to run this test, run it manually after you've authenticated with AWS"]
    async fn test_is_authenticated_with_aws() {
        assert!(is_authenticated_with_aws().await);
    }
}
