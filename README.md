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
1. Spinning up clusters (Provisioned mode only)
2. Listing all available clusters as well as their statuses (Provisioned mode only)
3. Submitting jobs to a cluster (Both Provisioned and BYOC modes)
4. Connecting to the cluster (Provisioned mode only)
5. Spinning down clusters (Provisioned mode only)
6. Creating configuration files (Both modes)
7. Running raw SQL statements (BYOC mode only)

## Operation Modes

Daft Launcher supports two modes of operation:
- **Provisioned**: Automatically provisions and manages Ray clusters in AWS
- **BYOC (Bring Your Own Cluster)**: Connects to existing Ray clusters in Kubernetes

### Command Groups and Support Matrix

| Command Group | Command | Provisioned | BYOC |
|--------------|---------|-------------|------|
| cluster      | up      | ✅          | ❌   |
|              | down    | ✅          | ❌   |
|              | kill    | ✅          | ❌   |
|              | list    | ✅          | ❌   |
|              | connect | ✅          | ❌   |
|              | ssh     | ✅          | ❌   |
| job          | submit  | ✅          | ✅   |
|              | sql     | ✅          | ❌   |
|              | status  | ✅          | ❌   |
|              | logs    | ✅          | ❌   |
| config       | init    | ✅          | ✅   |
|              | check   | ✅          | ❌   |
|              | export  | ✅          | ❌   |

## Usage

### Pre-requisites

You'll need some python package manager installed.
We recommend using [`uv`](https://astral.sh/blog/uv) for all things python.

#### For Provisioned Mode (AWS)
1. A valid AWS account with the necessary IAM role to spin up EC2 instances.
   This IAM role can either be created by you (assuming you have the appropriate permissions)
   or will need to be created by your administrator.
2. The [AWS CLI](https://aws.amazon.com/cli/) installed and configured on your machine.
3. Login using the AWS CLI.

#### For BYOC Mode (Kubernetes)
1. A Kubernetes cluster with Ray already deployed
   - Can be local (minikube/kind), cloud-managed (EKS/GKE/AKS), or on-premise.
   - See our [BYOC setup guides](./docs/byoc/README.md) for detailed instructions
2. Ray cluster running in your Kubernetes cluster
   - Must be installed and configured using Helm
   - See provider-specific guides for installation steps
3. Daft installed on the Ray cluster
4. `kubectl` installed and configured with the correct context
5. Appropriate permissions to access the namespace where Ray is deployed

### SSH Key Setup for Provisioned Mode

To enable SSH access and port forwarding for provisioned clusters, you need to:

1. Create an SSH key pair (if you don't already have one):
   ```bash
   # Generate a new key pair
   ssh-keygen -t rsa -b 2048 -f ~/.ssh/daft-key
   
   # This will create:
   #   ~/.ssh/daft-key     (private key)
   #   ~/.ssh/daft-key.pub (public key)
   ```

2. Import the public key to AWS:
   ```bash
   # Import the public key to AWS
   aws ec2 import-key-pair \
     --key-name "daft-key" \
     --public-key-material fileb://~/.ssh/daft-key.pub
   ```

3. Set proper permissions on your private key:
   ```bash
   chmod 600 ~/.ssh/daft-key
   ```

4. Update your daft configuration to use this key:
   ```toml
   [setup.provisioned]
   # ... other config ...
   ssh-private-key = "~/.ssh/daft-key"  # Path to your private key
   ssh-user = "ubuntu"                   # User depends on the AMI (ubuntu for Ubuntu AMIs)
   ```

Notes:
- The key name in AWS must match the name of your key file (without the extension)
- The private key must be readable only by you (hence the chmod 600)
- Different AMIs use different default users:
  - Ubuntu AMIs: use "ubuntu"
  - Amazon Linux AMIs: use "ec2-user"
  - Make sure this matches your `ssh-user` configuration

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

### Example Usage

All interactions with Daft Launcher are primarily communicated via a configuration file.
By default, Daft Launcher will look inside your `$CWD` for a file named `.daft.toml`.
You can override this behaviour by specifying a custom configuration file.

#### Provisioned Mode (AWS)

```bash
# Initialize a new provisioned mode configuration
daft config init --provider provisioned
# or use the default provider (provisioned)
daft config init

# Cluster management
daft provisioned up
daft provisioned list
daft provisioned connect
daft provisioned ssh
daft provisioned down
daft provisioned kill

# Job management (works in both modes)
daft job submit example-job
daft job status example-job
daft job logs example-job

# Configuration management
daft config check
daft config export
```

#### BYOC Mode (Kubernetes)

```bash
# Initialize a new BYOC mode configuration
daft config init --provider byoc
```

### Configuration Files

You can specify a custom configuration file path with the `-c` flag:
```bash
daft -c my-config.toml job submit example-job
```

Example Provisioned mode configuration:
```toml
[setup]
name = "my-daft-cluster"
version = "0.1.0"
provider = "provisioned"
dependencies = []  # Optional additional Python packages to install

[setup.provisioned]
region = "us-west-2"
number-of-workers = 4
ssh-user = "ubuntu"
ssh-private-key = "~/.ssh/daft-key"
instance-type = "i3.2xlarge"
image-id = "ami-04dd23e62ed049936"
iam-instance-profile-name = "YourInstanceProfileName"  # Optional

[run]
pre-setup-commands = []
post-setup-commands = []

[[job]]
name = "example-job"
command = "python my_script.py"
working-dir = "~/my_project"
```

Example BYOC mode configuration:
```toml
[setup]
name = "my-daft-cluster"
version = "0.1.0"
provider = "byoc"
dependencies = []  # Optional additional Python packages to install

[setup.byoc]
namespace = "default"  # Optional, defaults to "default"

[[job]]
name = "example-job"
command = "python my_script.py"
working-dir = "~/my_project"
```
