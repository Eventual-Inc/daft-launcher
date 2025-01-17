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
```
That should create a configuration file for you which should look like:

```toml
# This is a default configuration file that you can use to spin up a ray-cluster using `daft-launcher`.
# Change up some of the configurations in here, and then run `daft up`.
#
# For more information on the availale commands and configuration options, visit [here](https://eventual-inc.github.io/daft-launcher).
#
# Happy daft-ing!

[setup]
name = "daft-launcher-example"
version = "<VERSION>"
provider = "aws"
region = "us-west-2"
number-of-workers = 4

# The following configurations specify the type of servers in your cluster.
# The machine type below is what we usually use at Eventual, and the image id is Ubuntu based.
# If you want a smaller or bigger cluster, change the below two configurations accordingly.
instance-type = "i3.2xlarge"
image-id = "ami-04dd23e62ed049936"

# This is the user profile that ssh's into the head machine.
# This value depends upon the `image-id` value up above.
# For Ubuntu AMIs, keep it as 'ubuntu'; for AWS AMIs, change it to 'ec2-user'.
ssh-user = "ubuntu"

# Fill this out with your custom `.pem` key, or generate a new one by running `ssh-keygen -t rsa -b 2048 -m PEM -f my-key.pem`.
# Make sure the public key is uploaded to AWS.
ssh-private-key = "~/.ssh/my-keypair.pem"

# Fill in your python dependencies here.
# They'll be downloaded using `uv`.
dependencies = []
```

Some of the above values will need to be modified by you.
If you have any confusions on a value, you can always run `daft check` to check the syntax and schema of your configuration file.

Once you're content with your configuration file, go back to your terminal and run the following:

```sh
# spin your cluster up
daft up

# list all the active clusters
daft list

# submit a directory and command to run on the cluster
# (where `my-job-name` should be an entry in your .daft.toml file)
daft submit my-job-name

# run a direct SQL query on daft
daft sql "SELECT * FROM my_table WHERE column = 'value'"

# finally, once you're done, spin the cluster down
daft down
```
