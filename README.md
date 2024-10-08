<div align="center">
  <img src="https://emojis.wiki/thumbs/emojis/rocket.webp" alt="Daft Launcher">
</div>

<br>

[![PyPI Package](https://github.com/Eventual-Inc/daft-launcher/actions/workflows/publish-to-pypi.yaml/badge.svg)](https://github.com/Eventual-Inc/daft-launcher/actions/workflows/publish-to-pypi.yaml)
[![Deploy mdBook](https://github.com/Eventual-Inc/daft-launcher/actions/workflows/deploy-mdbook.yaml/badge.svg)](https://github.com/Eventual-Inc/daft-launcher/actions/workflows/deploy-mdbook.yaml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Latest](https://img.shields.io/github/v/tag/Eventual-Inc/daft-launcher?label=latest&logo=GitHub)](https://github.com/Eventual-Inc/daft-launcher/tags)
[![License](https://img.shields.io/badge/daft_launcher-docs-red.svg)](https://eventual-inc.github.io/daft-launcher)

# Daft Launcher CLI Tool

`daft-launcher` is a simple launcher for spinning up and managing Ray clusters for [`daft`](https://github.com/Eventual-Inc/Daft).

## Goal

Getting started with Daft in a local environment is easy.
However, getting started with Daft in a cloud environment is substantially more difficult.
So much more difficult, in fact, that users end up spending more time setting up their environment than actually playing with our query engine.

Daft Launcher aims to solve this problem by providing a simple CLI tool to remove all of this unnecessary heavy-lifting.

## Capabilities

What Daft Launcher is capable of:
1. Spinning up clusters.
2. Listing all available clusters (as well as their statuses).
3. Submitting jobs to a cluster.
4. Connecting to the cluster (to view the Ray dashboard and submit jobs using the Ray protocol).
5. Spinning down clusters.
6. Creating configuration files.
7. Running raw SQL statements using Daft's SQL API.

## Currently supported cloud providers

- [x] AWS
- [ ] GCP
- [ ] Azure

## Usage

### Pre-requisites

You'll need some python package manager installed.
We recommend using `uv` for all things python.

#### AWS

If you're using AWS, you'll need:
1. A valid AWS account with the necessary IAM role to spin up EC2 instances.
  This IAM role can either be created by you (assuming you have the appropriate permissions).
  Or this IAM role will need to be created by your administrator.
2. The [AWS CLI](https://aws.amazon.com/cli/) installed and configured on your machine.
3. To login using the AWS CLI.
  For full instructions, please look [here](https://google.com).

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

All interactions with Daft Launcher are primarily communicated via a configuration file.
By default, Daft Launcher will look inside your `$CWD` for a file named `.daft.toml`.
You can override this behaviour by specifying a custom configuration file.

```bash
# create a new configuration file
# will create a file named `.daft.toml` in the current working directory
daft init-config
# or optionally, pass in a custom name
daft init-config my-custom-config.toml

# spin up a cluster
daft up
# or optionally, pass in a custom config file
daft up -c my-custom-config.toml

# list all the active clusters (can have multiple clusters running at the same time)
daft list

# submit a directory and a command to run on the cluster
daft submit --working-dir <...> -- command arg1 arg2 ...
# or optionally, pass in a custom config file
daft submit -c my-custom-config.toml --working-dir $WORKING_DIR -- command arg1 arg2 ...

# run a direct SQL query against the daft query engine running in the remote cluster
daft sql -- "SELECT * FROM my_table WHERE column = 'value'"
# or optionally, pass in a custom config file
daft sql -c my-custom-config.toml -- "SELECT * FROM my_table WHERE column = 'value'"

# spin down a cluster
daft down
# or optionally, pass in a custom name
daft down -c my-custom-config.toml
```
