# Init Config

This command is, in essence, the entrypoint to using daft-launcher.
This will initialize an empty configuration file, named `.daft.toml`, in the current working directory.
The file itself will contain some default values that you can tune to your liking.
Some of the values are required, while others are optional; which ones are which will be denoted as such.

## Usage

```bash
# initialize the default .daft.toml configuration file
daft init-config

# or, if you want, specify a custom name
daft init-config my-custom-config.toml
```

The contents of the file will be roughly:

```toml
[setup]
name = "$NAME - required"
provider = "aws"
region = "$REGION - optional, defaults to us-west-2"
ssh_user = "$SSH_USER - optional, defaults to ec2-user"
number_of_workers = "$NUMBER_OF_WORKERS - optional, defaults to 2 worker nodes"
instance_type = "$INSTANCE_TYPE - optional, defaults to m7g.medium"
image_id = "$IMAGE_ID - optional, defaults to ami-01c3c55948a949a52"
dependencies = ["$DEPENDENCIES - optional, defaults to an empty list"]

[run]
setup_commands = ['echo "Hello, World!"']
```

Each field in the `setup` section is demarcated with a comment that states whether it is required or optional.
If it is optional, the default value that we use internally will be listed as well.
