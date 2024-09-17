from typing import Optional, List
from pathlib import Path
import click
import src.ray_default_configs
from src.configs import DEFAULT_AWS, get_final_config, merge_config_with_default, merge_name_with_default
from ray import job_submission, dashboard
from ray.autoscaler.sdk import (
    create_or_update_cluster,
    get_head_node_ip,
    teardown_cluster,
)
import subprocess
import json


def cliwrapper(func):
    @click.option(
        "-p",
        "--provider",
        required=False,
        type=click.STRING,
        help="The cloud provider to use.",
    )
    @click.option(
        "-n",
        "--name",
        required=False,
        type=click.STRING,
        help="The name of the cluster.",
    )
    @click.option(
        "-c",
        "--config",
        required=False,
        type=click.Path(exists=True),
        help="TOML configuration file.",
    )
    def wrapper(
        provider: Optional[str], name: Optional[str], config: Optional[Path], **args
    ):
        func(provider, name, config, **args)

    return wrapper


@click.command("up", help="Spin the cluster up.")
@cliwrapper
def up(provider: Optional[str], name: Optional[str], config: Optional[Path]):
    final_config = get_final_config(provider, name, config)
    create_or_update_cluster(
        final_config, no_restart=False, restart_only=False, no_config_cache=True
    )
    print(f"Head node IP: {get_head_node_ip(final_config)}")
    print("Successfully spun the cluster up.")


@click.command("list", help="List all running clusters.")
def list():
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
            "Reservations[*].Instances[*].{Instance:InstanceId,State:State.Name,Tags:Tags}",
        ],
        capture_output=True,
        text=True,
    )
    instance_groups = json.loads(result.stdout)
    state_to_name_map: dict[str, List[tuple]] = {}
    for instance_group in instance_groups:
        for instance in instance_group:
            assert "State" in instance
            assert "Tags" in instance
            state = instance["State"]
            instance_id: str = instance["Instance"]
            name = None
            node_type = None
            for tag in instance["Tags"]:
                if tag["Key"] == "ray-node-type":
                    node_type = tag["Value"]
                if tag["Key"] == "ray-cluster-name":
                    name = tag["Value"]
            assert name is not None
            assert node_type is not None
            if state in state_to_name_map:
                state_to_name_map[state].append((name, instance_id, node_type))
            else:
                state_to_name_map[state] = [(name, instance_id, node_type)]
    for state, data in state_to_name_map.items():
        print(f"{state.capitalize()}:")
        for name, instance_id, node_type in data:
            formatted_name = f""
            print(f"\t - {name}, {node_type}, {instance_id}")


@click.command("submit", help="Submit a job.")
@cliwrapper
def submit(provider: Optional[str], config: Optional[Path]):
    pass


@click.command("down", help="Spin the cluster down.")
@cliwrapper
def down(provider: Optional[str], name: Optional[str], config: Optional[Path]):
    final_config = get_final_config(provider, name, config)
    teardown_cluster(final_config)
    print("Successfully spun the cluster down.")
