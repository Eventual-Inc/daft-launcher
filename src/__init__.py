from typing import Optional
from pathlib import Path
import click
import src.ray_default_configs
from src.ray_default_configs import merge_config_with_default
from ray.autoscaler.sdk import create_or_update_cluster


@click.command("up", help="Spin the cluster up.")
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
def up(provider: Optional[str], config: Optional[Path]):
    if provider and config:
        raise click.UsageError("Please provide either a provider or a config file.")
    elif provider:
        if provider == "aws":
            final_config = src.ray_default_configs.DEFAULT_AWS
        else:
            raise click.UsageError(f"Cloud provider {provider} not found")
    elif config:
        final_config = merge_config_with_default(config)
    else:
        raise click.UsageError("Please provide either a provider or a config file.")

    create_or_update_cluster(
        final_config, no_restart=False, restart_only=False, no_config_cache=True
    )
