# Daft Launcher CLI Tool

Simple command line tool to get up and running with `daft` using a cloud-provider.

## Currently supported clouds providers

- [x] AWS
- [ ] GCP
- [ ] Azure

## Example

```bash
# spin up a cluster
daft up $TOML_CONFIG_FILE

# list all the active clusters (can have multiple clusters running at the same time)
daft list

# spin down a cluster
daft down $TOML_CONFIG_FILE
```
