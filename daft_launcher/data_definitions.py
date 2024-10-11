"""Data definitions for the ray cluster setup.
Defines how the .daft.toml file should be structured.

The primary entrypoint into this module is the `build_ray_config_from_path` function.
"""

import ray
import sys
import click
from dataclasses import dataclass, field
from typing import Literal, Optional, Union, Any, List
from pathlib import Path

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib


RayConfiguration = dict[str, Any]


def _determine_python_version() -> str:
    maj = sys.version_info.major
    min = sys.version_info.minor
    mic = sys.version_info.micro
    return f"{maj}.{min}.{mic}"


@dataclass
class Setup:
    name: str
    provider: Literal["aws"]
    python_version: str = field(default_factory=_determine_python_version)
    ray_version: str = field(default=ray.__version__)
    number_of_workers: int = field(default=2)
    dependencies: List[int] = field(default_factory=list)


@dataclass
class Run:
    pre_setup_commands: List[str] = field(default_factory=list)
    setup_commands: List[str] = field(default_factory=list)


@dataclass
class Configuration:
    setup: Setup
    run: Run = field(default_factory=Run)


@dataclass
class AwsConfiguration(Configuration):
    region: str = field(default="us-west-2")
    ssh_user: str = field(default="ec2-user")
    instance_type: str = field(default="m7g.medium")
    image_id: str = field(default="ami-01c3c55948a949a52")
    iam_instance_profile_arn: Optional[str] = None


def _generate_setup_commands(
    config: Configuration,
) -> List[str]:
    return [
        "curl -LsSf https://astral.sh/uv/install.sh | sh",
        f"uv python install {config.setup.python_version}",
        f"uv python pin {config.setup.python_version}",
        "uv venv",
        """echo "alias pip='uv pip'" >> $HOME/.bashrc""",
        'echo "source $HOME/.venv/bin/activate" >> $HOME/.bashrc',
        "source $HOME/.bashrc",
        f'uv pip install "ray[default]=={config.setup.ray_version}" "getdaft" "deltalake"',
    ]


def _construct_config_from_raw_dict(custom_config: dict[str, Any]) -> Configuration:
    if "setup" not in custom_config:
        raise click.UsageError("No setup section found in config file.")
    if "provider" not in custom_config["setup"]:
        raise click.UsageError(
            "No provider value found in the setup section in the config file."
        )
    provider = custom_config["setup"]["provider"]
    if provider == "aws":
        setup = Setup(**custom_config["setup"])
        run = Run(**custom_config["run"])
        return AwsConfiguration(setup=setup, run=run)
    else:
        raise click.UsageError(f"Cloud provider {provider} not found")


def _construct_config_from_path(custom_config_path: Path) -> Configuration:
    try:
        with open(custom_config_path, "rb") as stream:
            custom_config = tomllib.load(stream)
            return _construct_config_from_raw_dict(custom_config)
    except click.UsageError as ce:
        raise ce
    except TypeError as te:
        (arg,) = te.args
        error = str(arg).removeprefix("Setup.__init__() g")
        raise click.UsageError(f"G{error}")
    except Exception as arg:
        raise click.UsageError(f"Error reading config file {custom_config_path}")


def _build_ray_config(
    custom_config: Configuration,
) -> RayConfiguration:
    if custom_config.setup.provider == "aws":
        aws_custom_config: AwsConfiguration = custom_config  # type: ignore
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
                "ray.head.default": {
                    "resources": {},
                    "node_config": {
                        "InstanceType": aws_custom_config.instance_type,
                        "ImageId": aws_custom_config.image_id,
                    },
                },
                "ray.worker.default": {
                    "resources": {},
                    "node_config": {
                        "InstanceType": aws_custom_config.instance_type,
                        "ImageId": aws_custom_config.image_id,
                    },
                    "min_workers": aws_custom_config.setup.number_of_workers,
                    "max_workers": aws_custom_config.setup.number_of_workers,
                },
            },
            "setup_commands": aws_custom_config.run.pre_setup_commands
            + _generate_setup_commands(aws_custom_config)
            + aws_custom_config.run.setup_commands,
        }
    else:
        raise Exception("unreachable")


def build_ray_config_from_path(custom_config_path: Path) -> RayConfiguration:
    """Takes in a path to a file and returns a RayConfiguration object.

    # Assumptions:
    Assumes the path is a valid path to a file that exists.
    If it does not, the error printed to the console will be slightly misleading.
    Please check for existence beforehand.
    """

    custom_config = _construct_config_from_path(custom_config_path)
    return _build_ray_config(custom_config)
