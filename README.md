<div align="center">
  <img src="https://emojis.wiki/thumbs/emojis/rocket.webp" alt="Daft Launcher">
</div>

<br>

[![PyPI Package](https://github.com/Eventual-Inc/daft-launcher/actions/workflows/publish-to-pypi.yaml/badge.svg)](https://github.com/Eventual-Inc/daft-launcher/actions/workflows/publish-to-pypi.yaml)
[![Deploy mdBook](https://github.com/Eventual-Inc/daft-launcher/actions/workflows/deploy-mdbook.yaml/badge.svg)](https://github.com/Eventual-Inc/daft-launcher/actions/workflows/deploy-mdbook.yaml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Latest](https://img.shields.io/github/v/tag/Eventual-Inc/daft-launcher?label=latest&logo=GitHub)](https://github.com/Eventual-Inc/daft-launcher/tags)
[![License](https://img.shields.io/badge/daft_launcher-docs-red.svg)](https://eventual-inc.github.io/daft-launcher)

# Daft Launcher

`daft-launcher` is a simple launcher for spinning up and managing Ray clusters for [`daft`](https://github.com/Eventual-Inc/Daft).
It abstracts away all the complexities of dealing with Ray yourself, allowing you to focus on running `daft` in a distributed manner.

## Capabilities

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

You'll need a python package manager installed.
We highly recommend using [`uv`](https://astral.sh/blog/uv) for all things python!

### AWS

If you're using AWS, you'll need:
1. A valid AWS account with the necessary IAM role to spin up EC2 instances.
  This IAM role can either be created by you (assuming you have the appropriate permissions).
  Or this IAM role will need to be created by your administrator.
2. The [AWS CLI](https://aws.amazon.com/cli) installed and configured on your machine.
3. To login using the AWS CLI.
  For full instructions, please look [here](https://google.com).

## Installation

Using `uv` (recommended):

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

## Example

```sh
# create a new configuration file
daft init

# modify the values inside that configuration file

# check your configuration file is valid
daft check

# if everything is good, spin your cluster up
daft up

# list all the active clusters
daft list

# submit a directory and command to run on the cluster
daft submit --working-dir $WORKING_DIR -- command arg0 arg1 ...

# run a direct SQL query on daft
daft sql -- 'SELECT * FROM my_table WHERE column = "value"'

# finally, once you're done, spin the cluster down
daft down
```
