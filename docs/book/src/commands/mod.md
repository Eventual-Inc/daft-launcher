# Commands

Daft-launcher currently exposes 6 commands to interface with and manage your cluster.
They are:

1. `daft init-config`
2. `daft up`
3. `daft down`
4. `daft list`
5. `daft submit`
6. `daft dashboard`

Succinctly, the idea is that you are able to list clusters (`list`), start new clusters (`up`), and tear down existing clusters (`down`).
You are also able to submit jobs to the cluster (`submit`) and view the dashboard of a given cluster (`dashboard`).
The dashboard gives you the ability to access the ray web ui, which gives you additional information into statuses on the cluster and current/past jobs.
Finally, as a convenience, you are also able to initialize a configuration file (`init-config`) that is pre-populated with some configuration options that will be used by the other commands.

The above 6 commands will hopefully be complete over your usage of quickly experimenting with `daft` in a distributed environment.
Let's dive into each command individually.
We'll explain the high-level overview of that given command, as well as some basic examples of how to use it.
