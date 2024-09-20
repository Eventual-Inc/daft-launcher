from typing import Optional, List, Any
from pathlib import Path
import click
from . import configs
from ray.autoscaler import sdk as ray_sdk
import subprocess
import json
import ray
from ray import job_submission
import time
import requests


def ssh_command(ip: str) -> list[str]:
    return [
        "ssh",
        "-N",
        "-L",
        "8265:localhost:8265",
        "-i",
        "/Users/rabh/.ssh/ray-autoscaler_5_us-west-2.pem",
        f"ec2-user@{ip}",
    ]


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


def cliwrapper(func):
    @click.argument(
        "config",
        required=False,
        type=click.Path(exists=True),
    )
    def wrapper(
        config: Optional[str],
        **args,
    ):
        if config:
            config_path = Path(config).absolute()
            if not config_path.exists():
                raise click.UsageError(
                    f"Config file does not exist at path '{config_path}'."
                )
        else:
            config_path = Path(".daft-launcher.toml").absolute()
            if not config_path.exists():
                raise click.UsageError(
                    f"No default '.daft.toml' file found in current directory."
                )
        if not config_path.is_file():
            raise click.UsageError(f"The path '{config_path}' is not a file.")
        func(config_path, **args)

    return wrapper


@click.command("up", help="Spin the cluster up.")
@cliwrapper
def up(config: Path):
    final_config = configs.get_merged_config(config)
    ray_sdk.create_or_update_cluster(
        final_config,
        no_restart=False,
        restart_only=False,
        no_config_cache=True,
    )
    print(f"Head node IP: {ray_sdk.get_head_node_ip(final_config)}")
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
            "Reservations[*].Instances[*].{Instance:InstanceId,State:State.Name,Tags:Tags,Ip:PublicIpAddress}",
        ],
        capture_output=True,
        text=True,
    )
    instance_groups = json.loads(result.stdout)
    state_to_name_map: dict[str, List[tuple]] = {}
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
            if state in state_to_name_map:
                state_to_name_map[state].append((name, instance_id, node_type, ip))
            else:
                state_to_name_map[state] = [(name, instance_id, node_type, ip)]
    for state, data in state_to_name_map.items():
        print(f"{state.capitalize()}:")
        for name, instance_id, node_type, ip in data:
            formatted_name = f""
            print(
                f"\t - {name}, {node_type}, {instance_id}" + (f", {ip}" if ip else "")
            )


@click.command(
    "dashboard",
    help="Enable port-forwarding between an existing cluster and your local machine; required before the submission of jobs.",
)
@cliwrapper
def dashboard(
    config: Path,
):
    subprocess.run(
        ssh_command(get_ip(config)),
        close_fds=True,
        capture_output=False,
        text=False,
    )


@click.command("submit", help="Submit a job.")
@click.option(
    "--working-dir",
    required=False,
    default=".",
    type=click.Path(exists=True),
    help="Submit a python working dir as a job.",
)
@click.option(
    "--cmd",
    required=False,
    default="python3 main.py",
    type=click.STRING,
    help="Submit a python working dir as a job.",
)
@cliwrapper
def submit(
    config: Path,
    working_dir: str,
    cmd: str,
):
    process = subprocess.Popen(
        ssh_command(get_ip(config)),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        working_dir_path = Path(working_dir).absolute()
        client = None
        tries = 0
        max_tries = 3
        while tries <= max_tries:
            try:
                client = job_submission.JobSubmissionClient("http://localhost:8265")
                break
            except Exception as e:
                tries += 1
                if tries >= max_tries:
                    raise e
                time.sleep(1)
        assert client
        id = client.submit_job(
            entrypoint=cmd,
            runtime_env={
                "working_dir": working_dir_path.absolute(),
            }
            if working_dir
            else None,
        )
        print(f"Job ID: {id}")
    finally:
        process.terminate()


@click.command("down", help="Spin the cluster down.")
@cliwrapper
def down(config: Path):
    final_config = configs.get_merged_config(config)
    ray_sdk.teardown_cluster(final_config)
    print("Successfully spun the cluster down.")


@click.group()
def cli():
    pass


def main():
    cli.add_command(up)
    cli.add_command(list)
    cli.add_command(submit)
    cli.add_command(dashboard)
    cli.add_command(down)
    cli()
