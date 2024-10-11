import asyncio
from typing import List, Optional, Any
from pathlib import Path
import subprocess
import json
from . import configs
import click


def ssh_helper(
    final_config: dict,
    identity_file: Path,
    additional_port_forwards: list[int] = [],
) -> subprocess.Popen[str]:
    process = subprocess.Popen(
        ssh_command(
            ip=get_ip(final_config),
            user=final_config['auth']["ssh_user"],
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


def list_helper() -> dict[str, List[tuple]]:
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
            "Reservations[*].Instances[*].{Instance:InstanceId,State:State.Name,Tags:Tags,Ip:PublicIpAddress}",
        ]
    )
    state_to_cluster_info_map: dict[str, List[tuple]] = {}
    for instance_group in instance_groups:
        for instance in instance_group:
            state = instance["State"]
            instance_id: str = instance["Instance"]
            ip: str = instance["Ip"]
            name = None
            node_type = None
            for tag in instance["Tags"]:
                if tag["Key"] == "ray-node-type":
                    node_type = tag["Value"]
                if tag["Key"] == "ray-cluster-name":
                    name = tag["Value"]
            assert name is not None
            assert node_type is not None
            if state in state_to_cluster_info_map:
                state_to_cluster_info_map[state].append(
                    (name, instance_id, node_type, ip)
                )
            else:
                state_to_cluster_info_map[state] = [(name, instance_id, node_type, ip)]
    return state_to_cluster_info_map


def get_region(final_config: dict) -> str:
    if "provider" not in final_config:
        raise click.UsageError("The provider field is required in the configuration.")
    if "region" not in final_config["provider"]:
        raise click.UsageError("The region field is required in the provider field.")
    return final_config["provider"]["region"]


def get_instance_id(final_config: dict) -> str:
    name = final_config["cluster_name"]
    state_to_cluster_info_map = list_helper()
    if "running" not in state_to_cluster_info_map:
        raise click.UsageError(
            f"The cluster {name} is not running; cannot connect to it."
        )
    if len(state_to_cluster_info_map["running"]) == 0:
        raise click.UsageError(
            f"The cluster {name} is not running; cannot connect to it."
        )
    for _, instance_id, node_type, _ in state_to_cluster_info_map["running"]:
        if node_type == "head":
            return instance_id
    raise click.UsageError("Could not find the head node's Instance-Id.")


def query_for_public_keypair(final_config: dict) -> Optional[str]:
    region = get_region(final_config)
    instance_id = get_instance_id(final_config)
    keys: List[List[Any]] = run_aws_command(
        [
            "aws",
            "ec2",
            "describe-instances",
            "--region",
            region,
            "--instance-ids",
            instance_id,
            "--query",
            "Reservations[*].Instances[*].KeyName",
        ],
    )
    assert len(keys) == 1
    instance_keys = keys[0]
    num_of_keys = len(instance_keys)
    if num_of_keys == 0:
        return None
    elif num_of_keys == 1:
        return instance_keys[0]
    else:
        raise click.ClickException("This instance has multiple public key-pairs.")


def detect_keypair(final_config: dict) -> Path:
    if public_keypair_name := query_for_public_keypair(final_config):
        path = Path("~").expanduser() / ".ssh" / f"{public_keypair_name}.pem"
        if path.exists():
            return path
        else:
            raise click.ClickException(
                f"The public keypair name of the head node is called {public_keypair_name}, but no private keypair of that same name was found in the ~/.ssh directory; please re-run this command and manually pass in the path to the private keypair using the `-i <PATH_TO_KEY_PAIR>` flag."
            )
    else:
        raise click.UsageError(
            "Could not detect keypair; please manually specify one by using the `-i <PATH_TO_KEY_PAIR>` flag."
        )


def get_ip(final_config: dict):
    name = final_config["cluster_name"]
    region = get_region(final_config)
    instance_groups: List[List[Any]] = run_aws_command(
        [
            "aws",
            "ec2",
            "describe-instances",
            "--region",
            region,
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


def ssh_command(
    ip: str,
    user: str | None,
    pub_key: Optional[Path] = None,
    additional_port_forwards: list[int] = [],
) -> list[str]:
    additional_port_forward_args = [
        arg
        for args in map(
            lambda pf: ["-L", f"{pf}:localhost:{pf}"], additional_port_forwards
        )
        for arg in args
    ]
    return (
        [
            "ssh",
            "-N",
            "-L",
            "8265:localhost:8265",
        ]
        + additional_port_forward_args
        + (["-i", str(pub_key)] if pub_key else [])
        + [
            f"{user}@{ip}" if user else f"ec2-user@{ip}",
        ]
    )


async def print_logs(logs):
    async for lines in logs:
        print(lines, end="")


async def wait_on_job(logs):
    await asyncio.wait_for(print_logs(logs), timeout=None)
