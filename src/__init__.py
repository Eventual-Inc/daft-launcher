from typing import Optional
from pathlib import Path
import click


@click.command("up", help="Spin the cluster up.")
@click.option(
    '--provider',
    required=False,
    type=click.STRING,
    help="The cloud provider to use.",
)
@click.option(
    "--config",
    required=False,
    type=click.Path(exists=True),
    help="TOML configuration file.",
)
def up(provider: Optional[str], config: Optional[Path]):
    if provider and config:
        raise Exception("Please provide either a provider or a config file.")
    elif provider:
        print(provider)
    elif config:
        print(config)
    else:
        raise Exception("Please provide either a provider or a config file.")
