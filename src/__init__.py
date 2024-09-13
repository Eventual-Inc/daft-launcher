from typing import Optional
from pathlib import Path
import click
import src.ray_default_configs
from src.configs import DEFAULT_AWS, merge_config_with_default
from ray.autoscaler.sdk import (
    create_or_update_cluster,
    get_head_node_ip,
    teardown_cluster,
)


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


@click.command("down", help="Spin the cluster down.")
@cliwrapper
def down(provider: Optional[str], config: Optional[Path]):
    final_config = get_final_config(provider, config)
    teardown_cluster(final_config)
    print("Successfully spun the cluster down.")
