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
