from typing import List, Optional, Any
from pathlib import Path
import subprocess
from . import configs, helpers
from ray.autoscaler import sdk as ray_sdk
from ray import job_submission
import click
import time


AWS_TEMPLATE_PATH = Path(__file__).parent / "templates" / "aws.toml"


def init_config(name: Path):
    with open(AWS_TEMPLATE_PATH) as template_f:
        with open(name, "w") as config_f:
            config_f.write(template_f.read())


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
    state_to_name_map = helpers.list_helper()
    for state, data in state_to_name_map.items():
        print(f"{state.capitalize()}:")
        for name, instance_id, node_type, ip in data:
            formatted_name = f""
            print(
                f"\t - {name}, {node_type}, {instance_id}" + (f", {ip}" if ip else "")
            )


def connect(
    config: Path,
    identity_file: Optional[Path],
):
    final_config = configs.get_merged_config(config)
    if not identity_file:
        identity_file = helpers.detect_keypair(final_config)

    subprocess.run(
        helpers.ssh_command(helpers.get_ip(final_config), identity_file, additional_port_forwards=[10001]),
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
    final_config = configs.get_merged_config(config)
    if not identity_file:
        identity_file = helpers.detect_keypair(final_config)
    cmd = " ".join([arg for arg in cmd_args])

    process = subprocess.Popen(
        helpers.ssh_command(helpers.get_ip(final_config), identity_file),
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
