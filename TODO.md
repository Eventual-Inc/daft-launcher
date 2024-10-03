# Hard blocks

- [ ] Lower python pinned version
  Doesn't need to be pinned to 3.12
- [ ] Enable users to specify Ray version
  Currently, when running remotely, if the local Ray version and the remote Ray version are different, the user will run into issues.

# Medium blocks

- [ ] Create new `daft sql` command.
  Should take in a SQL string, use argparse to parse the sql string, upload and run that working dir using `daft submit`.
- [ ] Quality of life improvements to up:
  Blocking API call (does *not* immediately return once the head node is up)

# Soft blocks
- [ ] Open up default browser, create new tab, and point it towards `localhost:8265` after running `daft connect`
- [ ] Understand how public IPs / public DNSs are provisioned upon spin-up
- [ ] Debug the absence of bindings of the NVME drives upon initialization of the cluster nodes.

# Future todos

- [ ] Add ability to upload a custom `daft` .whl when initializing a cluster.
  This would greatly improve iteration speeds.
