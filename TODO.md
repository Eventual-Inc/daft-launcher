# Todos
- [ ] Perform automatic detection of the local private SSH keypair file.
  This should be based off of the remote cluster's head's public keypair *name*.
  (The remote cluster's head node should be able to be queried in order to grab its public keypair name).

- [ ] Quality of life improvements to submission:
  - [ ] Forward stdout printouts from the remote cluster to local during the execution of a job.
  - [ ] Automatic detection of private keypair file.
  - [ ] Update submission such that it will error if all worker nodes are not initialized yet.

- [ ] Quality of life improvements to dashboard command:
  - [ ] Try to default open the browser upon invocation of the command.
  - [ ] Printouts of what is happening.
    Right now, nothing is being printed out!
    That just makes things tough for the end-user!

- [ ] Debug the absence of bindings of the NVME drives upon initialization of the cluster nodes.

- [ ] understand how public IPs / public DNSs are provisioned upon spin-up
