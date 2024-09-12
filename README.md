# Daft CLI Tool

Simple command line tool to get up and running with `daft` using a cloud-provider.

## Currently supported clouds providers

- [x] AWS
- [ ] GCP
- [ ] Azure

## Example

```bash
# spin up a cluster
daft up --name my-cluster --cloud aws --region us-west-2 --nodes 3

# list all the active clusters (can have multiple clusters running at the same time)
daft list

# spin down a cluster
daft down --name my-cluster
```
