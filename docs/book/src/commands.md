# Commands

Daft-launcher currently exposes 6 commands to interface with and manage your cluster.
They are:

1. [`daft init-config`](#init-config)
2. [`daft up`](#up)
3. [`daft down`](#down)
4. [`daft list`](#list)
5. [`daft submit`](#submit)
6. [`daft dashboard`](#dashboard)

Succinctly, the idea is that you are able to list clusters (`list`), start new clusters (`up`), and tear down existing clusters (`down`).
You are also able to submit jobs to the cluster (`submit`) and view the dashboard of a given cluster (`dashboard`).
The dashboard gives you the ability to access the ray web ui, which gives you additional information into statuses on the cluster and current/past jobs.
Finally, as a convenience, you are also able to initialize a configuration file (`init-config`) that is pre-populated with some configuration options that will be used by the other commands.

Let's dive into each command individually.

## Init Config

This command is, in essence, the entrypoint to using daft-launcher.
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

## Up

This command spins up a cluster given some configuration file.
The configuration file itself will contain all of the information that daft-launcher will require in order to know *how* to spin that specific cluster up.

### Example

```bash
# spin up a cluster using the default .daft.toml configuration file
daft up

# or, if you want, spin up a cluster using a custom configuration file
daft up -c my-custom-config.toml
```

This command will do a couple of things:
  1. Firstly, it will reach into your cloud provider and spin up the necessary resources.
  This includes things such as the worker nodes, security groups, permissions, etc.
  2. When the nodes are spun up, the ray and daft dependencies will be downloaded into a python virtual environment.
  3. Next, any other custom dependencies that you've specified in the configuration file will then be downloaded.
  4. Finally, the setup commands that you've specified in the configuration file will be run on the head node.

The command will only return successfully when the head node is fully set up.
Even though this command *will* request the worker nodes to also spin up, it will not wait for them to be spun up before returning.
Therefore, when the command completes, and you type in `daft list`, the worker nodes may be in a "pending" state.
Don't be concerned; they should, in a couple of seconds, be fully running.

## Down

The down command is pretty much the opposite of the up command.
It takes the cluster specified in the configuration file and tears it down.

### Example

```bash
# spin down a cluster using the default .daft.toml configuration file
daft down

# or, if you want, spin down a cluster using a custom configuration file
daft down -c my-custom-config.toml
```

This command will tear down *all* instances in the cluster, not just the head node.
When each instance has been requested to shut down, the command will return successfully.

## List

The list command is extremely helpful for getting some observability into the current state of all of your clusters.
List will return a formatted table of all of the clusters that you currently have, running *and* terminated.
It will tell you each of their instance names, as well as their public IPs (given that they are still running).

### Example

```bash
daft list
```

An example output after running the above command would be:

```txt
Running:
  - daft-demo, head, i-053f9d4856d92ea3d, 35.94.91.91
  - daft-demo, worker, i-00c340dc39d54772d
  - daft-demo, worker, i-042a96ce1413c1dd6
```

The name of the cluster which was booted up is "daft-demo".
The cluster is comprised of 3 instances: 1 head node and 2 worker nodes.

The list command can output multiple clusters as well.
For example, let's say I created another configuration file and spun up a new cluster using it.

```bash
daft init-config new-cluster.toml
daft up -c new-cluster.toml
```

Then, after running `daft list`, the output would be:

```txt
Running:
  - daft-demo, head, i-053f9d4856d92ea3d, 35.94.91.91
  - daft-demo, worker, i-00c340dc39d54772d, 44.234.112.173
  - daft-demo, worker, i-042a96ce1413c1dd6, 35.94.206.130
  - new-cluster, head, i-0be0db9803bd06652, 35.86.200.101
  - new-cluster, worker, i-056f46bd69e1dd3f1, 44.242.166.108
  - new-cluster, worker, i-09ff0e1d8e67b8451, 35.87.221.180
```

Now, let's say I terminated the new cluster using `daft down -c new-cluster.toml`.
Then, after running `daft list`, the output would be:

```txt
Running:
  - daft-demo, head, i-053f9d4856d92ea3d, 35.94.91.91
  - daft-demo, worker, i-00c340dc39d54772d, 44.234.112.173
  - daft-demo, worker, i-042a96ce1413c1dd6, 35.94.206.130
Shutting-down:
  - new-cluster, head, i-0be0db9803bd06652, 35.86.200.101
  - new-cluster, worker, i-056f46bd69e1dd3f1, 44.242.166.108
  - new-cluster, worker, i-09ff0e1d8e67b8451, 35.87.221.180
```

The state of the new-cluster has changed from "Running" to "Shutting-down".
In a couple seconds, the state should then be finalized to "Terminated".

## Submit

The submit command enables you submit a working directory and command to the remote cluster in order to be run.
The working directory will be zipped prior to being sent over the wire, and then will be unzipped on the remote head node.

An important thing to keep in mind is how dependencies are utilized by the source code in the working directory.
During the initial `daft up` command that you ran, the dependencies should have been specified in the configuration file.
During the cluster's initialization process, the cluster will download all the dependencies into a python virtual environment.
The working directory that you submit will then be run in that virtual environment, thus enabling it to access those pre-downloaded dependencies.

### Example

```bash
# submit a job using the default .daft.toml configuration file
daft submit -i my-keypair.pem -w my-working-director

# submit a job using the default .daft.toml configuration file
daft submit -c my-custom-config.toml -i my-keypair.pem -w my-working-director
```

## Dashboard

The dashboard command enables you to view the Ray dashboard of a specified cluster that you currently have running.
The way this is done is by establishing a port-forward over SSH from your local machine to the head node of the cluster (connecting `localhost:8265` to the remote head's `8265`).
The head node then serves some HTML/CSS/JS that you can view in your browser by pointing it towards `localhost:8265`).

An important thing to note is that this command will require that you have the appropriate private SSH keypair to authenticate against the remote head's public SSH keypair.
You will need to pass this SSH keypair as an argument to the command.

### Example

```bash
# establish the port-forward using the default .daft.toml configuration file
daft dashboard -i my-keypair.pem

# or, if you want, establish the port-forward using a custom configuration file
daft dashboard -c my-custom-config.toml -i my-keypair.pem
```
