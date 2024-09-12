import click
from pathlib import Path
import daft


@click.group()
def cli():
    pass


def main():
    cli.add_command(daft.up)
    cli()


if __name__ == "__main__":
    main()
