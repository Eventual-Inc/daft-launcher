import asyncio
from botocore.exceptions import TokenRetrievalError
from typing import List, Optional, Any
from pathlib import Path
import subprocess
from . import helpers, data_definitions
from ray.autoscaler import sdk as ray_sdk
from ray import job_submission
import click
import time


AWS_TEMPLATE_PATH = Path(__file__).parent / "assets" / "aws.toml"
ON_CONNECTION_MESSAGE = """Successfully connected to Ray Cluster!

To view your cluster dashboard, navigate to: http://localhost:8265.

To run Daft against your Ray cluster, use the following code snippet:

```
import daft

daft.context.set_runner_ray(address="ray://localhost:10001")

df = daft.from_pydict({"foo": [1, 2, 3], "bar": [4, 5, 6]})
df.show()
```

Happy daft-ing! 🚀"""


def init_config(name: Path):
    with open(AWS_TEMPLATE_PATH) as template_f:
        with open(name, "w") as config_f:
            config_f.write(template_f.read())
            print(f"Successfully created a new configuration file: {name}")


def up(ray_config: data_definitions.RayConfiguration):
    ray_sdk.create_or_update_cluster(
        ray_config,
        no_restart=False,
        restart_only=False,
        no_config_cache=True,
    )
    print(f"Head node IP: {ray_sdk.get_head_node_ip(ray_config)}")
    print("Successfully spun the cluster up.")


def list():
    state_map = helpers.list_helper()
    for state_index, (state, instance_infos) in enumerate(state_map.items()):
        if state_index != 0:
            print()
        print(f"{state.replace('-', ' ').capitalize()}:")
        for index, instance_info in enumerate(instance_infos):
            if index != 0:
                print()
            print(str(instance_info))


def connect(
    ray_config: data_definitions.RayConfiguration,
    identity_file: Optional[Path],
):
    if not identity_file:
        identity_file = helpers.detect_keypair(ray_config)
    process = helpers.ssh_helper(ray_config, identity_file, [10001])
    print(ON_CONNECTION_MESSAGE)
    process.wait()


def submit(
    ray_config: data_definitions.RayConfiguration,
    identity_file: Optional[Path],
    working_dir: Path,
    cmd_args: List[str],
):
    if not identity_file:
        identity_file = helpers.detect_keypair(ray_config)
    cmd = " ".join(cmd_args)

    process = helpers.ssh_helper(ray_config, identity_file)
    try:
        working_dir_path = Path(working_dir).absolute()
        client = None
        tries = 0
        max_tries = 10
        while tries <= max_tries:
            try:
                client = job_submission.JobSubmissionClient(
                    address="http://localhost:8265"
                )
                break
            except Exception as e:
                tries += 1
                if tries >= max_tries:
                    raise e
                time.sleep(0.1)
        assert client
        job_id = client.submit_job(
            entrypoint=cmd,
            runtime_env={
                "working_dir": working_dir_path.absolute(),
            }
            if working_dir
            else None,
        )
        print(f"Job ID: {job_id}")

        asyncio.run(helpers.wait_on_job(client.tail_job_logs(job_id)))
        status = client.get_job_status(job_id)
        assert status.is_terminal(), "Job should have terminated"
        job_info = client.get_job_info(job_id)
        print(f"Job completed with {status}")

    finally:
        process.terminate()


def sql(
    ray_config: data_definitions.RayConfiguration,
    identity_file: Optional[Path],
    cmd_args: List[str],
):
    submit(
        ray_config,
        identity_file,
        Path(__file__).parent / "assets",
        ["python", "sql.py"] + cmd_args,
    )


def down(ray_config: data_definitions.RayConfiguration):
    ray_sdk.teardown_cluster(ray_config)
    print("Successfully spun the cluster down.")
