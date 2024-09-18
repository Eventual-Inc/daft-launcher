# Blocking
- take a look into how iam arns are provisioned (especially when nothing is provided)
  - add docs into how users can either provision this manually or pass roles to EC2 machine
  - Permissions:
    1. spin up ec2 machines
    2. access s3
    3. pass role permissions
- figure out how to package and distribute this `daft` cli tool

# Cherries on top
- edit `daft down` to take in a name and provider instead of a path to a config file
  - the down command (I think) just uses those two pieces of information anyways
- print out job-id to the terminal after submitting a job
- instead of `daftcli submit --cmd '...'`, have `daftcli submit -- ...`
- have the submit API hang and open a stdio connection between the remote and local
  - easy for viewing logs while the process runs in remote
- Add more examples and helpful docs to the README.md
- edit submission to error out if not all workers are alive and pass message to end-user
- edit list such that all cluster nodes are together
  - add some indentation (head is at the top; workers are one line indented; have some "aggregate" stats on workers status)
- understand how public IPs / public DNSs are provisioned upon spin-up
- add dependencies that are downloadable via `uv pip install`
