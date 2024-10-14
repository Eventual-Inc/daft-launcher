"""Data definitions for the ray cluster setup.
Defines how the .daft.toml file should be structured.

The primary entrypoint into this module is the `build_ray_config_from_path` function.
"""

import ray
import sys
from typing import Literal, Optional, Union, Any, List
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


class PreConfiguredTemplates(BaseModel):
    type: Union[
        Literal["light"],
        Literal["normal"],
        Literal["gpus"],
    ]


class Run(BaseModel):
    pre_setup_commands: List[str] = Field(default_factory=list)
    setup_commands: List[str] = Field(default_factory=list)


class CustomConfiguration(BaseModel):
    daft_launcher_version: str
    setup: Setup
    pre_configured_templates: Optional[PreConfiguredTemplates] = Field(default=None)
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
