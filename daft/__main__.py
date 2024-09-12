import ray
from ray.autoscaler.sdk import create_or_update_cluster, teardown_cluster, get_head_node_ip
import click
from pathlib import Path


cluster_config_path = Path(__file__).parent / 'ray_cfg' / 'graviton.yaml'


@click.command('up', help='Spin up the cluster')
def up():
    create_or_update_cluster(str(cluster_config_path), no_restart=False, restart_only=False, no_config_cache=True)
    head_node_ip = get_head_node_ip(str(cluster_config_path))
    print('Cluster spun up successfully')
    print(f"Ray cluster head node IP: {head_node_ip}")


@click.command('down', help='Spin down the cluster')
def down():
    teardown_cluster(str(cluster_config_path))
    print('Cluster spun down successfully')


@click.group()
def cli():
    pass


def main():
    cli.add_command(up)
    cli.add_command(down)
    cli()
