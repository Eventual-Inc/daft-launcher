from typing import Optional, List
from pathlib import Path
import click
import src.ray_default_configs
from src import configs
from ray.autoscaler import sdk as ray_sdk
import subprocess
import json
import ray
from ray import job_submission, dashboard


def cliwrapper(func):
    @click.option(
        "-c",
        "--config",
        required=False,
        type=click.Path(exists=True),
        help="TOML configuration file.",
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
            config_path = Path(".daft.toml").absolute()
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
            assert "Instance" in instance
            assert "State" in instance
            assert "Tags" in instance
            assert "Ip" in instance
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
    "connect",
    help="Enable port-forwarding between an existing cluster and your local machine; required before the submission of jobs.",
)
def connect():
    # subprocess.run(
    #     [
    #         "ssh",
    #         "-N",
    #         "-L",
    #         "8265:localhost:8265",
    #         "-i",
    #         "/Users/rabh/.ssh/ray-autoscaler_5_us-west-2.pem",
    #         "ec2-user@35.86.59.3",
    #     ],
    #     close_fds=True,
    #     capture_output=False,
    #     text=False,
    # )
    ...


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
    working_dir_path = Path(working_dir).absolute()
    client = job_submission.JobSubmissionClient("http://localhost:8265")
    client.submit_job(
        entrypoint=cmd,
        runtime_env={
            "working_dir": working_dir_path.absolute(),
        }
        if working_dir
        else None,
    )


@click.command("down", help="Spin the cluster down.")
@cliwrapper
def down(config: Path):
    final_config = configs.get_merged_config(config)
    ray_sdk.teardown_cluster(final_config)
    print("Successfully spun the cluster down.")
