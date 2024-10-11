from typing import Optional
import click
from pathlib import Path
from . import commands, data_definitions
from importlib import metadata


DEFAULT_CONFIG_PATH = Path(".daft.toml")


def generate_intro_message():
    summary = metadata.metadata("daft-launcher").get("Summary")
    info_string = "For more documentation, please visit:\n\nhttps://eventual-inc.github.io/daft-launcher"
    return f"{summary}\n\n{info_string}"


def get_config_path(config: Optional[Path]) -> data_definitions.RayConfiguration:
    if config:
        if not config.exists():
            raise click.UsageError("Config file does not exist.")
    else:
        config = DEFAULT_CONFIG_PATH
        if not config.exists():
            raise click.UsageError(
                f"No default '{DEFAULT_CONFIG_PATH}' file found in current directory."
            )
    return data_definitions.build_ray_config_from_path(config)


def assert_identity_file_path(identity_file: Optional[Path]):
    if not identity_file:
        return
    if not identity_file.exists():
        raise click.UsageError("Identity file does not exist.")


def assert_working_dir(working_dir: Path):
    if not working_dir.exists():
        raise click.UsageError("Working dir does not exist.")
    if not working_dir.is_dir():
        raise click.UsageError("Working dir must be a directory.")


def get_new_configuration_file_path(name: Optional[Path]) -> Path:
    name = name or DEFAULT_CONFIG_PATH
    if name.is_file():
        raise click.UsageError(f"A configuration file at path {name} already exists.")
    elif name.is_dir():
        raise click.UsageError(
            "That is the path to a directory; please pass in a file name."
        )
    elif name.exists():
        raise click.UsageError("That path already exists; please use a new one.")

    return name


def identity_file_option(func):
    return click.option(
        "--identity-file",
        "-i",
        required=False,
        type=Path,
        help="Path to the identity file.",
    )(func)


def working_dir_option(func):
    return click.option(
        "--working-dir",
        "-w",
        required=True,
        type=Path,
        help="Path to the working directory.",
    )(func)


def init_config_file_name_argument(func):
    return click.argument(
        "name",
        required=False,
        type=Path,
    )(func)


def config_option(func):
    return click.option(
        "--config",
        "-c",
        required=False,
        type=Path,
        help=f"Path to the configuration file; defaults to {DEFAULT_CONFIG_PATH}.",
    )(func)


def cmd_args_argument(func):
    return click.argument("cmd_args", nargs=-1, type=click.UNPROCESSED, required=True)(
        func
    )


def init_config_command(func):
    return click.command("init-config", help="Create a new configuration file.")(func)


def up_command(func):
    return click.command("up", help="Spin the cluster up.")(func)


def down_command(func):
    return click.command("down", help="Spin the cluster down.")(func)


def list_command(func):
    return click.command("list", help="List all running clusters.")(func)


def connect_command(func):
    return click.command(
        "connect",
        help="Enable port-forwarding between a cluster and your local machine.",
    )(func)


def submit_command(func):
    return click.command("submit", help="Submit a job to the specified cluster.")(func)


def sql_command(func):
    return click.command("sql", help="Submit a SQL query to the specified cluster.")(
        func
    )


@init_config_command
@init_config_file_name_argument
def init_config(name: Optional[Path]):
    name = get_new_configuration_file_path(name)
    commands.init_config(name)


@up_command
@config_option
def up(config: Optional[Path]):
    ray_config = get_config_path(config)
    commands.up(ray_config)


@list_command
def list():
    commands.list()


@connect_command
@config_option
@identity_file_option
def connect(
    config: Optional[Path],
    identity_file: Optional[Path],
):
    ray_config = get_config_path(config)
    assert_identity_file_path(identity_file)
    commands.connect(ray_config, identity_file)


@submit_command
@config_option
@working_dir_option
@identity_file_option
@cmd_args_argument
def submit(
    config: Optional[Path],
    identity_file: Optional[Path],
    working_dir: Path,
    cmd_args: tuple[str],
):
    ray_config = get_config_path(config)
    assert_identity_file_path(identity_file)
    assert_working_dir(working_dir)
    cmd_args_list = [arg for arg in cmd_args]
    commands.submit(ray_config, identity_file, working_dir, cmd_args_list)


@sql_command
@config_option
@identity_file_option
@cmd_args_argument
def sql(
    config: Optional[Path],
    identity_file: Optional[Path],
    cmd_args: tuple[str],
):
    ray_config = get_config_path(config)
    assert_identity_file_path(identity_file)
    cmd_args_list = [arg for arg in cmd_args]
    commands.sql(ray_config, identity_file, cmd_args_list)


@down_command
@config_option
def down(config: Optional[Path]):
    ray_config = get_config_path(config)
    commands.down(ray_config)


@click.group(help=generate_intro_message())
@click.version_option(version=metadata.version("daft-launcher"))
def cli(): ...


def run_cli():
    cli.add_command(init_config)
    cli.add_command(up)
    cli.add_command(list)
    cli.add_command(connect)
    cli.add_command(submit)
    cli.add_command(sql)
    cli.add_command(down)
    cli()
