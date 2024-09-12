import ray
from ray.autoscaler.sdk import create_or_update_cluster, teardown_cluster, get_head_node_ip
import click


cluster_config_path = "daft/ray_cfg/graviton.yaml"


@click.command('down', help='Tear down the cluster')
def down():
    teardown_cluster(cluster_config_path)


@click.command('up', help='Spin up the cluster')
def up():
    # Path to your cluster configuration file

    # Create or update the cluster
    create_or_update_cluster(cluster_config_path, no_restart=False, restart_only=False, no_config_cache=True)

    # Get the head node's IP address (useful for connecting to the cluster programmatically)
    head_node_ip = get_head_node_ip(cluster_config_path)
    print(f"Ray cluster head node IP: {head_node_ip}")


@click.group()
def cli():
    pass


def main():
    cli.add_command(up)
    cli.add_command(down)
    cli()
