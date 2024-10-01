# Hard blocks
- [ ] Perform automatic detection of the local private SSH keypair file.
  This should be based off of the remote cluster's head's public keypair *name*.
  (The remote cluster's head node should be able to be queried in order to grab its public keypair name).
  - [ ] Or add a new field inside of the TOML file that specifies the private keypair file.
  - [ ] Overridable with `-i`.

- [ ] Quality of life improvements to submission:
  - [ ] Forward stdout printouts from the remote cluster to local during the execution of a job.
  - [ ] Automatic detection of private keypair file during submission as well.

- [ ] Print a nice message when running `daft connect`.
  Let the user know that something has happened.

# Medium blocks

- [ ] Create new `daft sql` command.
  Should take in a SQL string, use argparse to parse the sql string, upload and run that working dir using `daft submit`.

- [ ] Quality of life improvements to up:
  Blocking API call (does *not* immediately return once the head node is up)

# Soft blocks
- [ ] Avoid pinning python to 3.12.
- Open up default browser, create new tab, and point it towards `localhost:8265` after running `daft connect`
- [ ] understand how public IPs / public DNSs are provisioned upon spin-up
- [ ] Debug the absence of bindings of the NVME drives upon initialization of the cluster nodes.

# Future todos

- [ ] Add ability to upload a custom `daft` .whl when initializing a cluster.
  This would greatly improve iteration speeds.
