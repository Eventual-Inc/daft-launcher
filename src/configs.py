from pathlib import Path
import tomllib
from typing import Optional

import click
import yaml


DEFAULT_AWS = str(Path(__file__).parent / "ray_default_configs" / "aws.yaml")


def merge(custom: dict, default: dict) -> dict:
    if "name" in custom:
        default["cluster_name"] = custom["name"]
    if "region" in custom:
        default["provider"]["region"] = custom["region"]
    if "ssh_user" in custom:
        default["auth"]["ssh_user"] = custom["ssh_user"]

    if "workers" in custom:
        workers = custom["workers"]
        default["max_workers"] = workers
        default["available_node_types"]["ray.worker.default"]["min_workers"] = workers
        default["available_node_types"]["ray.worker.default"]["max_workers"] = workers

    if "instance_type" in custom:
        instance_type = custom["instance_type"]
        default["available_node_types"]["ray.head.default"]["node_config"][
            "InstanceType"
        ] = instance_type
        default["available_node_types"]["ray.worker.default"]["node_config"][
            "InstanceType"
        ] = instance_type

    if "image_id" in custom:
        image_id = custom["image_id"]
        default["available_node_types"]["ray.head.default"]["node_config"][
            "ImageId"
        ] = image_id
        default["available_node_types"]["ray.worker.default"]["node_config"][
            "ImageId"
        ] = image_id

    if "iam_instance_profile" in custom:
        default["available_node_types"]["ray.worker.default"]["node_config"][
            "IamInstanceProfiel"
        ]["Arn"] = custom["iam_instance_profile"]

    return default


def merge_config_with_default(
    config: Path,
) -> dict | str:
    with open(config, "rb") as stream:
        custom_config = tomllib.load(stream)
        if "provider" not in custom_config:
            raise click.UsageError(
                "Please provide a cloud provider in the config file."
            )

        provider = custom_config["provider"]

        if provider == "aws":
            with open(DEFAULT_AWS, "rb") as stream:
                default_aws_config = yaml.safe_load(stream)
                return merge(custom_config, default_aws_config)
        else:
            raise click.UsageError(f"Cloud provider {provider} not found")
