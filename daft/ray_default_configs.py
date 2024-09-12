from pathlib import Path
import tomllib
from typing import Optional

import yaml


DEFAULT_AWS = str(Path(__file__).parent / "ray_default_configs" / "aws_default.yaml")


def merge(custom: dict, default: dict) -> dict:
    if 'name' in custom:
        default['cluster_name'] = custom['name']

    return default


def merge_custom_with_default_aws(
    config: Optional[Path],
) -> dict | str:
    if config:
        with open(config, "rb") as stream:
            custom_config = tomllib.load(stream)
            with open(DEFAULT_AWS, "rb") as stream:
                default_aws_config = yaml.safe_load(stream)
                return merge(custom_config, default_aws_config)
    else:
        return DEFAULT_AWS
