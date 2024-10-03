<div align="center">
  <img src="https://emojis.wiki/thumbs/emojis/rocket.webp" alt="Daft Launcher" height="256">
</div>

<div style="display: flex; flex-direction: row;">
  <a href="https://github.com/Eventual-Inc/daft-launcher/actions/workflows/publish-to-pypi.yaml" style="padding: 0px 5px;">
    <img src="https://github.com/Eventual-Inc/Daft/actions/workflows/python-package.yml/badge.svg" alt="GitHub Actions Publishing to PyPI">
  </a>
  <a href="./LICENSE-MIT" style="padding: 0px 5px;">
    <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License">
  </a>
</div>

<br/>

# Daft Launcher CLI Tool

A simple launcher for spinning up and managing Ray clusters for Daft.

## Purpose

What `daft-launcher` is capable of:
1. Spinning up clusters.
2. Listing all available clusters (as well as their statuses).
3. Submitting jobs to a cluster.
4. Opening up a "dashboard process" to allow end-users to connect to a Ray dashboard to view metrics about their cluster.
5. Spinning down clusters.
6. Creating a default configuration file.
7. Running raw SQL statements using `daft`'s SQL API.

## Currently supported cloud providers

- [x] AWS
- [ ] GCP
- [ ] Azure

## Usage

### Pre-requisites

1. You will need a valid AWS account with the necessary IAM role to spin up EC2 instances.
  - This IAM role can either be created by you (assuming you have the appropriate permissions).
  - Or this IAM role will need to be created by your administrator.
2. You will need to have the AWS CLI installed and configured on your machine.
3. You will need to login using the AWS CLI. For full instructions, please look [here](https://google.com).

### Installation

Using `uv`:

```bash
# create project
mkdir my-project
cd my-project

# initialize project and setup virtual env
uv init
uv venv
source .venv/bin/activate

# install launcher
uv pip install daft-launcher
```

### Example

All interactions with `daft-launcher` are primarily communicated via a configuration file.
By default, `daft-launcher` will look inside your `$CWD` for a file named `.daft-launcher.toml`.
You can also specify a custom file by passing in the path to the configuration file as an argument, if you wish.

```bash
# create a new configuration file
# will create a file named `.daft-launcher.toml` in the current working directory
daft init-config --non-interactive

# spin up a cluster
daft up $CONFIG_FILE
# if you don't include $CONFIG_FILE, it will default to using `.daft-launcher.toml`
# e.g.: `daft up`

# list all the active clusters (can have multiple clusters running at the same time)
daft list

# submit a directory and a command to run on the cluster
daft submit $CONFIG_FILE --working-dir $WORKING_DIR -- command arg1 arg2 ...
# if you don't include $CONFIG_FILE, it will default to using `.daft-launcher.toml`
# e.g.: `daft submit --working-dir $WORKING_DIR -- command arg1 arg2 ...`

# spin down a cluster
daft down $CONFIG_FILE
# if you don't include $CONFIG_FILE, it will default to using `.daft-launcher.toml`
# e.g.: `daft down`
```
