# Commands

Daft launcher currently exposes 6 commands to interface with and manage your cluster.
They are:

1. [`daft init-config`](./init-config.md)
2. [`daft up`](./up.md)
3. [`daft down`](./down.md)
4. [`daft list`](./list.md)
5. [`daft submit`](./submit.md)
6. [`daft connect`](./connect.md)
7. [`daft sql`](./sql.md)

Succinctly, the idea is that you are able to list clusters (`list`), start new clusters (`up`), and tear down existing clusters (`down`).
You are also able to submit jobs to the cluster (`submit`) and view the dashboard of a given cluster (`connect`).
The dashboard gives you the ability to access the ray web ui, which gives you additional information into statuses on the cluster and current/past jobs.
Finally, as a convenience, you are also able to initialize a configuration file (`init-config`) that is pre-populated with some configuration options that will be used by the other commands.

Let's dive into each command individually.
