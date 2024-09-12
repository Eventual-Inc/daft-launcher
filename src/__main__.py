import click
from pathlib import Path
from . import up


@click.group()
def cli():
    pass


def main():
    cli.add_command(up)
    cli()


if __name__ == "__main__":
    main()
