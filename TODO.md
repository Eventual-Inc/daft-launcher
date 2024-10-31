# Hard blocks
- add GPU + Light instance profiles
- remove "us-west-2" hardcoding throughout the codebase
  - (find all instances by running `rg "us-west-2" src/`)

# Medium blocks
- quality of life improvements to up:
  - Blocking API call (does *not* immediately return once the head node is up)
- finish down command

# Soft blocks
- understand how public IPs / public DNSs are provisioned upon spin-up
- debug the absence of bindings of the NVME drives upon initialization of the cluster nodes
- clean up (in terminal) user selection logic
  - there are 2 user selection functions named `with_selection` and `with_selection_2`
  - remove the first one (and the associated trait that goes along with it)

# Future todos
- add ability to upload a custom `daft` wheel when initializing a cluster
  - this would greatly improve iteration speeds
