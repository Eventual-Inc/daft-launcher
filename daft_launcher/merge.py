from . import data_definitions
from typing import Any
from pathlib import Path


def _merge(
    custom_config: data_definitions.Configuration,
) -> data_definitions.RayConfiguration:
    if custom_config.setup.provider == "aws":
        aws_custom_config: data_definitions.AwsConfiguration = custom_config
        return {
            "cluster_name": aws_custom_config.setup.name,
            "provider": {
                "type": "aws",
                "region": aws_custom_config.region,
                "cache_stopped_nodes": False,
            },
            "auth": {
                "ssh_user": aws_custom_config.ssh_user,
            },
            "max_workers": aws_custom_config.setup.number_of_workers,
            "available_node_types": {
                "ray": {
                    "head": {
                        "default": {
                            "node_config": {
                                "InstanceType": aws_custom_config.instance_type,
                                "ImageId": aws_custom_config.image_id,
                            },
                        },
                    },
                    "worker": {
                        "default": {
                            "node_config": {
                                "InstanceType": aws_custom_config.instance_type,
                                "ImageId": aws_custom_config.image_id,
                            },
                            "min_workers": aws_custom_config.setup.number_of_workers,
                            "max_workers": aws_custom_config.setup.number_of_workers,
                        },
                    },
                },
            },
            "setup_commands": aws_custom_config.run.pre_setup_commands
            + aws_custom_config.run.setup_commands,
        }
    else:
        raise Exception("unreachable")


def merge_from_path(custom_config_path: Path) -> data_definitions.RayConfiguration:
    custom_config = data_definitions.construct_from_path(custom_config_path)
    return _merge(custom_config)
