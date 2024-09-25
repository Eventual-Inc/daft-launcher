# Blocking
- take a look into how iam arns are provisioned (especially when nothing is provided)
  - add docs into how users can either provision this manually or pass roles to EC2 machine
  - Permissions:
    1. spin up ec2 machines
    2. access s3
    3. pass role permissions

# Cherries on top
- have the submit API hang and open a stdio connection between the remote and local
  - easy for viewing logs while the process runs in remote
- Add more examples and helpful docs to the README.md
- edit submission to error out if not all workers are alive and pass message to end-user
- edit list such that all cluster nodes are together
  - add some indentation (head is at the top; workers are one line indented; have some "aggregate" stats on workers status)
- understand how public IPs / public DNSs are provisioned upon spin-up
