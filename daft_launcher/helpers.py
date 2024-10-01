from typing import List, Optional, Any
from pathlib import Path
import subprocess
import json
from . import configs
import click


def query_for_public_keypair() -> Optional[str]:
    run_aws_command(
        [
            "aws",
        ]
    )
    ...


def detect_keypair() -> Path:
    if public_keypair_name := query_for_public_keypair():
        ...
    else:
        raise click.UsageError(
            "Could not detect keypair; please manually specify one by using the `-i <PATH_TO_KEY_PAIR>` flag."
        )


def get_ip(final_config: dict):
    name = final_config["cluster_name"]
    instance_groups: List[List[Any]] = run_aws_command(
        [
            "aws",
            "ec2",
            "describe-instances",
            "--region",
            "us-west-2",
            "--filters",
            "Name=tag:ray-node-type,Values=*",
            "--query",
            "Reservations[*].Instances[*].{State:State.Name,Tags:Tags,Ip:PublicIpAddress}",
        ],
    )
    state_to_ips_mapping = find_ip(instance_groups, name)
    if "running" not in state_to_ips_mapping:
        raise click.UsageError(
            f"The cluster {name} is not running; cannot connect to it."
        )
    assert len(state_to_ips_mapping["running"]) <= 1
    if state_to_ips_mapping["running"]:
        ip = state_to_ips_mapping["running"][0]
    else:
        raise click.UsageError(
            f"The cluster {name} is not running; cannot connect to it."
        )
    if not ip:
        raise click.UsageError(
            f"The cluster {name} does not have a public IP address available."
        )
    return ip


def run_aws_command(args: list[str]) -> Any:
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


def find_ip(
    instance_groups: List[List[Any]], name: str
) -> dict[str, list[Optional[str]]]:
    ip = None
    state_to_ips_mapping: dict[str, list[Optional[str]]] = {}

    def insert(state: str, ip: Optional[str]):
        if state in state_to_ips_mapping:
            state_to_ips_mapping[state].append(ip)
        else:
            state_to_ips_mapping[state] = [ip]

    for instance_group in instance_groups:
        for instance in instance_group:
            is_head = False
            cluster_name = None
            state = instance["State"]
            for tag in instance["Tags"]:
                if tag["Key"] == "ray-cluster-name":
                    cluster_name = tag["Value"]
                elif tag["Key"] == "ray-node-type":
                    is_head = tag["Value"] == "head"
            if is_head and cluster_name == name:
                insert(state, instance["Ip"])

    if not state_to_ips_mapping:
        raise click.UsageError(
            f"The IP of the cluster with the name '{name}' not found."
        )

    return state_to_ips_mapping


# TODO!
# pass in a list of ports instead
def ssh_command(
    ip: str,
    pub_key: Optional[Path] = None,
    connect_10001: bool = False,
) -> list[str]:
    return (
        [
            "ssh",
            "-N",
            "-L",
            "8265:localhost:8265",
        ]
        + (["-L", "10001:localhost:10001"] if connect_10001 else [])
        + (["-i", str(pub_key)] if pub_key else [])
        + [
            f"ec2-user@{ip}",
        ]
    )
