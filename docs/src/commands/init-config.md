## Init Config

This command is, in essence, the entrypoint to using daft launcher.
This will initialize an empty configuration file, named `.daft.toml`, in the current working directory.
The file itself will contain some default values that you can tune to your liking.
Some of the values are required, while others are optional; which ones are which will be denoted as such.

### Example

```bash
# initialize the default .daft.toml configuration file
daft init-config

# or, if you want, specify a custom name
daft init-config my-custom-config.toml
```

## Config file specification

Each available configuration option is denoted below, as well as a small blurb on what it does and whether it is required or optional.
If it is optional, its default value will be defined as well.

```toml
[setup]

# (required)
# The name of the cluster.
name = ...

# (required)
# The cloud provider that this cluster will be created in.
# Has to be one of the following:
# - "aws"
# - "gcp"
# - "azure"
provider = ...

# (optional; default = None)
# The IAM instance profile ARN which will provide this cluster with the necessary permissions to perform whatever actions.
# Please note that if you don't specify this field, Ray will create an automatic instance profile for you.
# That instance profile will be minimal and may restrict some of the feature of Daft.
iam_instance_profile_arn = ...

# (required)
# The AWS region in which to place this cluster.
region = ...

# (optional; default = "ec2-user")
# The ssh user name when connecting to the cluster.
ssh_user = ...

# (optional; default = 2)
# The number of worker nodes to create in the cluster.
number_of_workers = ...

# (optional; default = "m7g.medium")
# The instance type to use for the head and worker nodes.
instance_type = ...

# (optional; default = "ami-01c3c55948a949a52")
# The AMI ID to use for the head and worker nodes.
image_id = ...

# (optional; default = [])
# A list of dependencies to install on the head and worker nodes.
# These will be installed using UV (https://docs.astral.sh/uv/).
dependencies = [...]

[run]

# (optional; default = ['echo "Hello, World!"'])
# Any post-setup commands that you want to invoke manually.
# This is a good location to install any custom dependencies or run some arbitrary script.
setup_commands = [...]
```
