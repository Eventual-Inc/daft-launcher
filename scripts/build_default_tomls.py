import tomllib
from importlib import metadata


def daft_launcher_version() -> str:
    return metadata.version("daft-launcher")


breakpoint()

# daft_launcher_version()
