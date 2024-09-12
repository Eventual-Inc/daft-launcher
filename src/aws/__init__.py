from typing import Optional
import tomllib
import yaml
import click
from pathlib import Path
from ray.autoscaler.sdk import (
    create_or_update_cluster,
    get_head_node_ip,
    teardown_cluster,
)
from daft.ray_default_configs import merge_custom_with_default_aws


@click.command("up", help="Spin the cluster up.")
@click.option(
    "--config",
    required=False,
    type=click.Path(exists=True),
    help="TOML configuration file.",
)
def up(config: Optional[Path]):
    final_config = merge_custom_with_default_aws(config)
    create_or_update_cluster(
        final_config,
        no_restart=False,
        restart_only=False,
        no_config_cache=True,
    )
    head_node_ip = get_head_node_ip(final_config)
    print(f"Ray cluster head node IP: {head_node_ip}")
    print("Successfully spun the cluster up.")


@click.command("list", help="List all the running clusters.")
def list():
    pass


@click.command("down", help="Spin the cluster down.")
def down():
    # teardown_cluster(DEFAULT_AWS)
    print("Successfully spun the cluster down.")


@click.group()
def aws():
    pass


aws.add_command(up)
aws.add_command(list)
aws.add_command(down)
