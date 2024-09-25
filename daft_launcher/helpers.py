from typing import List, Optional, Any
from pathlib import Path
import subprocess
import json
from . import configs
import click


def get_ip(config: Path):
    final_config = configs.get_merged_config(config)
    name = final_config["cluster_name"]
    result = subprocess.run(
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
        capture_output=True,
        text=True,
    )
    instance_groups = json.loads(result.stdout)
    ip, state = find_ip(instance_groups, name)
    if state != "running":
        raise click.UsageError(
            f"The cluster {name} is not running; cannot connect to it."
        )
    if not ip:
        raise click.UsageError(
            f"The cluster {name} does not have a public IP address available."
        )
    return ip


def find_ip(instance_groups: List[List[Any]], name: str) -> tuple[Optional[str], str]:
    ip = None
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
                return instance["Ip"], state
    raise click.UsageError(f"The IP of the cluster with the name '{name}' not found.")


def ssh_command(
    ip: str,
    pub_key: Optional[Path] = None,
) -> list[str]:
    return [
        "ssh",
        "-N",
        "-L",
        "8265:localhost:8265",
    ]
    +["-i", str(pub_key)] if pub_key else []
    +[
        f"ec2-user@{ip}",
    ]
