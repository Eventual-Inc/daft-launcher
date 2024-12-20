"""Data definitions for the ray cluster setup.
Defines how the .daft.toml file should be structured.

The primary entrypoint into this module is the `build_ray_config_from_path` function.
"""

import ray
import sys
import click
from typing import Literal, Optional, Union, Any, List
from pathlib import Path
from pydantic import BaseModel, Field, ValidationError

from daft_launcher import helpers

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


class Setup(BaseModel):
    name: str
    provider: Literal["aws"]
    python_version: str = Field(default_factory=_determine_python_version)
    ray_version: str = Field(default=ray.__version__)
    number_of_workers: int = Field(default=2)
    dependencies: List[int] = Field(default_factory=list)


class Run(BaseModel):
    pre_setup_commands: List[str] = Field(default_factory=list)
    setup_commands: List[str] = Field(default_factory=list)


class CustomConfiguration(BaseModel):
    daft_launcher_version: str
    setup: Setup
    run: Run = Field(default_factory=Run)

    def to_aws(self) -> Optional["AwsConfiguration"]:
        if self.setup.provider == "aws":
            aws_custom_config: AwsConfiguration = self  # type: ignore
            return aws_custom_config

    def force_to_aws(self) -> "AwsConfiguration":
        aws_custom_config = self.to_aws()
        assert aws_custom_config
        return aws_custom_config


class AwsConfiguration(CustomConfiguration):
    region: str = Field(default="us-west-2")
    ssh_user: str = Field(default="ec2-user")
    instance_type: str = Field(default="m7g.medium")
    image_id: str = Field(default="ami-01c3c55948a949a52")
    iam_instance_profile_arn: Optional[str] = Field(default=None)


ConfigurationBundle = tuple[CustomConfiguration, RayConfiguration]


def _generate_setup_commands(
    config: CustomConfiguration,
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


def _construct_config_from_raw_dict(
    custom_config: dict[str, Any],
) -> CustomConfiguration:
    if "setup" not in custom_config:
        raise click.UsageError("No setup section found in config file.")
    if "provider" not in custom_config["setup"]:
        raise click.UsageError(
            "No provider value found in the setup section in the config file."
        )
    provider = custom_config["setup"]["provider"]
    if provider == "aws":
        try:
            return AwsConfiguration(**custom_config)
        except ValidationError as validation_error:
            error_string = helpers.format_pydantic_validation_error(validation_error)
            raise click.UsageError(f"Error in config file: {error_string}")
    else:
        raise click.UsageError(f"Cloud provider {provider} not found")


def _construct_config_from_path(custom_config_path: Path) -> CustomConfiguration:
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
    custom_config: CustomConfiguration,
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


def build_ray_config_from_path(custom_config_path: Path) -> ConfigurationBundle:
    """Takes in a path to a file and returns a RayConfiguration object.

    # Assumptions:
    Assumes the path is a valid path to a file that exists.
    If it does not, the error printed to the console will be slightly misleading.
    Please check for existence beforehand.
    """

    custom_config = _construct_config_from_path(custom_config_path)
    toml_version = custom_config.daft_launcher_version
    launcher_version = helpers.daft_launcher_version()
    if toml_version != launcher_version:
        raise click.UsageError(f"Mismatch between launcher version and config file version; launcher version: {launcher_version}, config file version: {toml_version}")
    ray_config = _build_ray_config(custom_config)
    return custom_config, ray_config
