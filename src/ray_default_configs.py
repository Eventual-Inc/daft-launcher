from pathlib import Path
import tomllib
from typing import Optional

import click
import yaml


DEFAULT_AWS = str(Path(__file__).parent / "ray_default_configs" / "aws_default.yaml")


def merge(custom: dict, default: dict) -> dict:
    if "name" in custom:
        default["cluster_name"] = custom["name"]

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
