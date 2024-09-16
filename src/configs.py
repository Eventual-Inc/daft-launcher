from pathlib import Path
import tomllib
from typing import Optional

import click
import yaml


DEFAULT_AWS = str(Path(__file__).parent / "ray_default_configs" / "aws.yaml")


def merge(custom: dict, default: dict) -> dict:
    setup = custom["setup"]
    if "name" in setup:
        default["cluster_name"] = setup["name"]
    if "region" in setup:
        default["provider"]["region"] = setup["region"]
    if "ssh_user" in setup:
        default["auth"]["ssh_user"] = setup["ssh_user"]
    if "workers" in setup:
        workers = setup["workers"]
        default["max_workers"] = workers
        default["available_node_types"]["ray.worker.default"]["min_workers"] = workers
        default["available_node_types"]["ray.worker.default"]["max_workers"] = workers
    if "instance_type" in setup:
        instance_type = setup["instance_type"]
        default["available_node_types"]["ray.head.default"]["node_config"][
            "InstanceType"
        ] = instance_type
        default["available_node_types"]["ray.worker.default"]["node_config"][
            "InstanceType"
        ] = instance_type
    if "image_id" in setup:
        image_id = setup["image_id"]
        default["available_node_types"]["ray.head.default"]["node_config"][
            "ImageId"
        ] = image_id
        default["available_node_types"]["ray.worker.default"]["node_config"][
            "ImageId"
        ] = image_id
    if "iam_instance_profile" in setup:
        default["available_node_types"]["ray.worker.default"]["node_config"][
            "IamInstanceProfile"
        ]["Arn"] = setup["iam_instance_profile"]
    run = custom["run"]
    if "setup_commands" in run:
        setup_commands: list[str] = default["setup_commands"]
        setup_commands.extend(run["setup_commands"])

    return default


def merge_config_with_default(
    config: Path,
) -> dict | str:
    with open(config, "rb") as stream:
        custom_config = tomllib.load(stream)
        if "setup" not in custom_config:
            if "provider" not in custom_config["setup"]:
                raise click.UsageError(
                    "Please provide a cloud provider in the config file."
                )

        provider = custom_config["setup"]["provider"]

        if provider == "aws":
            with open(DEFAULT_AWS, "rb") as stream:
                default_aws_config = yaml.safe_load(stream)
                return merge(custom_config, default_aws_config)
        else:
            raise click.UsageError(f"Cloud provider {provider} not found")
