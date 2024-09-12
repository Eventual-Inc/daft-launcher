import click
from pathlib import Path
from . import up, down


@click.group()
def cli():
    pass


def main():
    cli.add_command(up)
    cli.add_command(down)
    cli()


if __name__ == "__main__":
    main()
