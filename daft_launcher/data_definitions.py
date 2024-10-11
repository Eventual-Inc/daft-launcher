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


@dataclass
class Setup:
    name: str
    provider: Literal["aws"]
    python_version: Optional[str] = None
    ray_version: Optional[str] = None
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


def _construct(custom_config: dict[str, Any]) -> Configuration:
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


def construct_from_path(custom_config_path: Path) -> Configuration:
    """Takes in a path to a file and returns a Configuration object.

    # Assumptions:
    Assumes the file exists.
    If it does not, the error printed to the console will be slightly misleading.
    Please check for existence beforehand.
    """

    try:
        with open(custom_config_path, "rb") as stream:
            custom_config = tomllib.load(stream)
            return _construct(custom_config)
    except click.UsageError as ce:
        raise ce
    except TypeError as te:
        (arg,) = te.args
        error = str(arg).removeprefix("Setup.__init__() g")
        raise click.UsageError(f"G{error}")
    except Exception as arg:
        raise click.UsageError(f"Error reading config file {custom_config_path}")
