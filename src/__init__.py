from typing import Optional, List
from pathlib import Path
import click
import src.ray_default_configs
from src.configs import DEFAULT_AWS, merge_config_with_default
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
        "-c",
        "--config",
        required=False,
        type=click.Path(exists=True),
        help="TOML configuration file.",
    )
    def wrapper(provider: Optional[str], config: Optional[Path]):
        func(provider, config)

    return wrapper


def get_final_config(
    provider: Optional[str],
    config: Optional[Path],
) -> dict | str:
    if provider and config:
        raise click.UsageError("Please provide either a provider or a config file.")
    elif provider:
        if provider == "aws":
            return DEFAULT_AWS
        else:
            raise click.UsageError(f"Cloud provider {provider} not found")
    elif config:
        return merge_config_with_default(config)
    else:
        raise click.UsageError("Please provide either a provider or a config file.")


@click.command("up", help="Spin the cluster up.")
@cliwrapper
def up(provider: Optional[str], config: Optional[Path]):
    final_config = get_final_config(provider, config)
    create_or_update_cluster(
        final_config, no_restart=False, restart_only=False, no_config_cache=True
    )
    print(f"Head node IP: {get_head_node_ip(final_config)}")
    print("Successfully spun the cluster up.")


@click.command("list", help="List all running clusters.")
@cliwrapper
def list(provider: Optional[str], config: Optional[Path]):
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
    [instances] = json.loads(result.stdout)
    state_to_name_map: dict[str, List[str]] = {}
    for instance in instances:
        assert "Tags" in instance
        assert "State" in instance
        state = instance["State"]
        for tag in instance["Tags"]:
            if tag["Key"] == "ray-cluster-name":
                name = tag["Value"]
                if state in state_to_name_map:
                    state_to_name_map[state].append(name)
                else:
                    state_to_name_map[state] = [name]
    for state, names in state_to_name_map.items():
        print(state.capitalize() + ": ")
        for name in names:
            print("\t - " + name)


@click.command("submit", help="Submit a job.")
@cliwrapper
def submit(provider: Optional[str], config: Optional[Path]):
    pass


@click.command("down", help="Spin the cluster down.")
@cliwrapper
def down(provider: Optional[str], config: Optional[Path]):
    final_config = get_final_config(provider, config)
    teardown_cluster(final_config)
    print("Successfully spun the cluster down.")
