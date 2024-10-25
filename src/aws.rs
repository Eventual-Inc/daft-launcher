use aws_config::{BehaviorVersion, Region};
use aws_sdk_ec2::{
    operation::describe_instances::DescribeInstancesOutput, Client,
};

use crate::{
    config::processed::{AwsCluster, ProcessedConfig},
    widgets::Spinner,
};

async fn is_authenticated_with_aws() -> bool {
    let sdk_config =
        aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(
                aws_config::meta::region::RegionProviderChain::default_provider(
                ),
            )
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
        spinner.fail();
        anyhow::bail!("You are not signed in to AWS; please sign in first")
    }
}

pub async fn list_instances(
    aws_cluster: &AwsCluster,
) -> anyhow::Result<DescribeInstancesOutput> {
    let region = Region::new(aws_cluster.region.to_string());
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .load()
        .await;
    let client = Client::new(&sdk_config);
    let output = client.describe_instances().send().await?;
    Ok(output)
}

pub async fn list_instance_names(
    aws_cluster: &AwsCluster,
) -> anyhow::Result<Vec<String>> {
    let instances = list_instances(aws_cluster).await?;
    let reservations = instances.reservations.unwrap_or_default();
    let mut instance_names = Vec::new();
    for instances in reservations
        .iter()
        .filter_map(|reservation| reservation.instances.as_ref())
    {
        for tags in instances
            .iter()
            .filter_map(|instance| instance.tags.as_ref())
        {
            for (key, value) in
                tags.iter().filter_map(|tag| tag.key().zip(tag.value()))
            {
                if key == "Name" {
                    instance_names.push(value.to_string());
                }
            }
        }
    }
    Ok(instance_names)
}

pub async fn instance_name_already_exists(
    processed_config: &ProcessedConfig,
    aws_cluster: &AwsCluster,
) -> anyhow::Result<bool> {
    let instance_names = list_instance_names(aws_cluster).await?;
    let is_contained = instance_names
        .contains(&format!("ray-{}-head", processed_config.package.name));
    Ok(is_contained)
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
