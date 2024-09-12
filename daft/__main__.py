import click
from pathlib import Path
from daft import aws


@click.group()
def cli():
    pass


def main():
    breakpoint()
    cli.add_command(aws.aws)
    cli()


if __name__ == "__main__":
    main()
