## Up

This command spins up a cluster given some configuration file.
The configuration file itself will contain all of the information that daft launcher will require in order to know *how* to spin that specific cluster up.

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
