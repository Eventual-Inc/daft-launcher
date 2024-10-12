"""Generic helpers.

Contain helpers for sshing, parsing outputs from `aws` commands, etc.

# Note
All helper/utility functions should go in here.
All core functions should go elsewhere.
"""

import asyncio
from dataclasses import dataclass, field
from typing import List, Literal, Optional, Any, Union
from pathlib import Path
import subprocess
import json
import click
from pydantic import BaseModel, Field, ValidationError

from daft_launcher import data_definitions


class Tag(BaseModel):
    Key: str
    Value: str


class InstanceInformation(BaseModel):
    instance_id: str
    iam_role: Optional[str]
    state: Union[
        Literal["terminated"], Literal["shutting-down"], Literal["running"], Any
    ]
    ip: Optional[str]
    keyname: str
    tags: List[Tag]

    def __str__(self) -> str:
        name = self.get_tag("ray-cluster-name")
        node_type = self.get_tag("ray-node-type")
        return f"""\tName: {name} ({node_type})
        Instance ID: {self.instance_id}
        IAM Role: {self.iam_role}
        State: {self.state}
        Ip: {self.ip}"""

    def get_tag(self, tag_name: str) -> Optional[str]:
        for tag in self.tags:
            if tag.Key == tag_name:
                return tag.Value


def _parse_describe_instances_query() -> List[InstanceInformation]:
    query_items = {
        "instance_id": "InstanceId",
        "iam_role": "IamInstanceProfile.Arn",
        "state": "State.Name",
        "tags": "Tags",
        "ip": "PublicIpAddress",
        "keyname": "KeyName",
    }
    query = ",".join([f"{key}:{value}" for key, value in query_items.items()])
    instance_groups: List[List[Any]] = _run_aws_command(
        [
            "aws",
            "ec2",
            "describe-instances",
            "--region",
            "us-west-2",
            "--filters",
            "Name=tag:ray-node-type,Values=*",
            "--query",
            f"Reservations[*].Instances[*].{{{query}}}",
        ]
    )
    def _parse_instance(instance: Any) -> InstanceInformation:
        try:
            return InstanceInformation(**instance)
        except ValidationError as e:
            raise click.ClickException(f'Failed to list clusters; failing on parsing {instance}')
    return [
        _parse_instance(instance)
        for instance_group in instance_groups
        for instance in instance_group
    ]


def _get_ip(config_bundle: data_definitions.ConfigurationBundle) -> str:
    custom_config, _ = config_bundle
    instance_infos = _parse_describe_instances_query()
    for instance_info in instance_infos:
        a = instance_info.state == "running"
        b = instance_info.get_tag("ray-node-type") == "head"
        c = instance_info.get_tag("ray-cluster-name") == custom_config.setup.name
        if a and b and c:
            assert instance_info.ip, "All running instances must have a public ip"
            return instance_info.ip
    raise click.UsageError(
        f"Could not find the public ip of {custom_config.setup.name}'s head node."
    )


def _get_public_keypair(
    config_bundle: data_definitions.ConfigurationBundle,
) -> str:
    custom_config, _ = config_bundle
    name = custom_config.setup.name
    instance_infos = _parse_describe_instances_query()
    for instance_info in instance_infos:
        a = instance_info.state == "running"
        b = instance_info.get_tag("ray-node-type") == "head"
        c = instance_info.get_tag("ray-cluster-name") == custom_config.setup.name
        if a and b and c:
            return instance_info.keyname
    raise click.UsageError(
        f"Could not find the public keypair of {custom_config.setup.name}'s head node."
    )


def _run_aws_command(args: list[str]) -> Any:
    result = subprocess.run(args, capture_output=True, text=True)
    if result.returncode != 0:
        if "Token has expired" in result.stderr:
            raise click.UsageError(
                "AWS token has expired. Please run `aws login`, `aws sso login`, or some other command to refresh it."
            )
    if result.stdout == "":
        raise click.UsageError(
            f"Failed to parse AWS command into json (empty response string)"
        )
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as e:
        raise click.UsageError(f"Failed to parse AWS command output: {result.stdout}")


def _ssh_command(
    config_bundle: data_definitions.ConfigurationBundle,
    pub_key: Optional[Path] = None,
    additional_port_forwards: list[int] = [],
) -> list[str]:
    custom_config, _ = config_bundle
    port_forwards = map(
        lambda pf: ["-l", f"{pf}:localhost:{pf}"], additional_port_forwards
    )
    port_forwards_args = [arg for args in port_forwards for arg in args]
    identity_args = ["-i", str(pub_key)] if pub_key else []
    user = custom_config.force_to_aws().ssh_user
    ip = _get_ip(config_bundle)
    return (
        [
            "ssh",
            "-N",
            "-L",
            "8265:localhost:8265",
        ]
        + port_forwards_args
        + identity_args
        + [f"{user}@{ip}"]
    )


def detect_keypair(config_bundle: data_definitions.ConfigurationBundle) -> Path:
    public_keypair_name = _get_public_keypair(config_bundle)
    path = Path("~").expanduser() / ".ssh" / f"{public_keypair_name}.pem"
    if path.exists():
        return path
    else:
        raise click.ClickException(
            f"The public keypair name of the head node is called {public_keypair_name}, but no private keypair of that same name was found in the ~/.ssh directory; please re-run this command and manually pass in the path to the private keypair using the `-i <PATH_TO_KEY_PAIR>` flag."
        )


def ssh(
    config_bundle: data_definitions.ConfigurationBundle,
    identity_file: Path,
    additional_port_forwards: list[int] = [],
) -> subprocess.Popen[str]:
    process = subprocess.Popen(
        _ssh_command(
            config_bundle,
            pub_key=identity_file,
            additional_port_forwards=additional_port_forwards,
        ),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if process.returncode and process.returncode != 0:
        raise click.ClickException(
            f"Failed to attach to the remote server. Return code: {process.returncode}"
        )
    else:
        assert process.stdout
        if process.stdout.readable():
            if text := process.stdout.read():
                print(text)
        else:
            raise click.ClickException(
                "Unable to establish a connection to the remote server."
            )
    return process


def get_state_map() -> dict[str, List[InstanceInformation]]:
    """Produce a state mapping of all the EC2 instances."""

    instance_infos = _parse_describe_instances_query()
    state_map = {}
    for instance_info in instance_infos:
        if instance_info.state not in state_map:
            state_map[instance_info.state] = []
        state_map[instance_info.state].append(instance_info)
    return state_map


async def print_logs(logs):
    async for lines in logs:
        print(lines, end="")


async def wait_on_job(logs):
    await asyncio.wait_for(print_logs(logs), timeout=None)


def format_pydantic_validation_error(validation_error: ValidationError) -> str:
    errors = validation_error.errors()
    assert errors
    def pull(error) -> tuple[str, str]:
        type = error['type']
        location = ".".join(error['loc'])
        return (type, f"`{location}`")
    error_string = ""
    for index, (type, error) in enumerate(map(pull, errors)):
        if index != 0:
            error_string = f"{error_string}, {type} {error}"
        else:
            error_string = f"{type} {error}"
    assert error_string, "`error_string` cannot be empty"
    return error_string
