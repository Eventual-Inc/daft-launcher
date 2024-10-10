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
