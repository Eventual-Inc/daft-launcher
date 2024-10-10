import sys

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib


def load_toml(path):
    return tomllib.load(path)


from pathlib import Path
from typing import Optional
import click
import yaml


RAY_DEFAULT_CONFIGS_PATH = Path(__file__).parent / "ray_default_configs"


def get_ray_config(
    provider: str,
) -> tuple[dict, list[tuple[str, list[list[str]], bool]]]:
    if provider == "aws":
        ray_config_path = RAY_DEFAULT_CONFIGS_PATH / "aws.yaml"
        setup_mappings = [
            ("name", [["cluster_name"]], False),
            ("provider", [["provider", "type"]], True),
            ("region", [["provider", "region"]], False),
            ("ssh_user", [["auth", "ssh_user"]], False),
            (
                "number_of_workers",
                [
                    ["max_workers"],
                    ["available_node_types", "ray.worker.default", "min_workers"],
                    ["available_node_types", "ray.worker.default", "max_workers"],
                ],
                False,
            ),
            (
                "instance_type",
                [
                    [
                        "available_node_types",
                        "ray.head.default",
                        "node_config",
                        "InstanceType",
                    ],
                    [
                        "available_node_types",
                        "ray.worker.default",
                        "node_config",
                        "InstanceType",
                    ],
                ],
                False,
            ),
            (
                "image_id",
                [
                    [
                        "available_node_types",
                        "ray.head.default",
                        "node_config",
                        "ImageId",
                    ],
                    [
                        "available_node_types",
                        "ray.worker.default",
                        "node_config",
                        "ImageId",
                    ],
                ],
                False,
            ),
            (
                "iam_instance_profile_arn",
                [
                    [
                        "available_node_types",
                        "ray.head.default",
                        "node_config",
                        "IamInstanceProfile",
                        "Arn",
                    ],
                    [
                        "available_node_types",
                        "ray.worker.default",
                        "node_config",
                        "IamInstanceProfile",
                        "Arn",
                    ],
                ],
                False,
            ),
        ]
    else:
        raise click.UsageError(f"Cloud provider {provider} not found")
    with open(ray_config_path, "rb") as stream:
        return yaml.safe_load(stream), setup_mappings


def get_custom_config(config_path: Path) -> tuple[dict, str]:
    with open(config_path, "rb") as stream:
        custom_config = load_toml(stream)
        if "setup" not in custom_config:
            raise click.UsageError("No setup section found in config file.")
            if "provider" not in custom_config["setup"]:
                raise click.UsageError(
                    "Please provide a cloud provider in the config file."
                )
        provider = custom_config["setup"]["provider"]
        return custom_config, provider


def merge(
    custom_config: dict,
    ray_config: dict,
    setup_mappings: list,
) -> dict:
    if "setup" not in custom_config:
        raise click.UsageError("No setup section found in config file.")
    setup: dict = custom_config["setup"]
    for a, b, required in setup_mappings:
        value = setup.get(a)
        if not value and required:
            raise click.UsageError(
                f"Required field {a} not found in custom config but is required."
            )
        elif not value:
            continue
        for b_i in b:
            y = ray_config
            for b_ii in b_i[:-1]:
                if b_ii not in y:
                    y[b_ii] = {}
                y = y[b_ii]
            y[b_i[-1]] = value

    setup_commands: list[str] = ray_config["setup_commands"]
    if "dependencies" in setup:
        quoted_dependencies = [f'"{dep}"' for dep in setup["dependencies"]]
        dependencies = " ".join(quoted_dependencies)
        uv_install_command = f"uv pip install {dependencies}"
        setup_commands.append(uv_install_command)
    if "run" in custom_config:
        if "pre_setup_commands" in custom_config["run"]:
            setup_commands = custom_config["run"]["pre_setup_commands"] + setup_commands
            ray_config["setup_commands"] = setup_commands
        if "setup_commands" in custom_config["run"]:
            setup_commands.extend(custom_config["run"]["setup_commands"])
    return ray_config


def get_merged_config(config_path: Path) -> dict:
    custom_config, provider = get_custom_config(config_path)
    ray_config, setup_mappings = get_ray_config(provider)
    return merge(custom_config, ray_config, setup_mappings)
