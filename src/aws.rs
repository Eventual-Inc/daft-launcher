use aws_config::{BehaviorVersion, Region};
use aws_sdk_ec2::{
    operation::describe_instances::DescribeInstancesOutput, Client,
};

use crate::config::processed::AwsCluster;

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

// #[cfg(test)]
// #[tokio::test]
// async fn test() {
//     let aws_cluster = AwsCluster {
//         region: "us-west-2".into(),
//         ssh_user: "".into(),
//         ssh_private_key: None,
//         image_id: "".into(),
//         instance_type: "".into(),
//         iam_instance_profile_arn: None,
//     };
//     list_instances(&aws_cluster).await.unwrap();
// }
