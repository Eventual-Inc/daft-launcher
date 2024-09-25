from typing import List, Optional, Any
from pathlib import Path
import subprocess
import json
from . import configs, helpers
from ray.autoscaler import sdk as ray_sdk
from ray import job_submission
import click
import time


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
    if result.returncode != 0:
        if "Token has expired" in result.stderr:
            raise click.UsageError(
                "AWS token has expired. Please run `aws login`, `aws sso login`, or some other command to refresh it."
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


def dashboard(
    config: Path,
    identity_file: Optional[Path],
):
    subprocess.run(
        helpers.ssh_command(
            helpers.get_ip(config), Path(identity_file) if identity_file else None
        ),
        close_fds=True,
        capture_output=False,
        text=False,
    )


def submit(
    config: Path,
    identity_file: Optional[Path],
    working_dir: Path,
    cmd_args: tuple[str],
):
    cmd = " ".join([arg for arg in cmd_args])
    process = subprocess.Popen(
        helpers.ssh_command(
            helpers.get_ip(config), Path(identity_file) if identity_file else None
        ),
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


def down(config: Path):
    final_config = configs.get_merged_config(config)
    ray_sdk.teardown_cluster(final_config)
    print("Successfully spun the cluster down.")
