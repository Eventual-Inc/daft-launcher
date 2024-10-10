## Submit

The submit command enables you submit a working directory and command to the remote cluster in order to be run.
The working directory will be zipped prior to being sent over the wire, and then will be unzipped on the remote head node.

An important thing to keep in mind is how dependencies are utilized by the source code in the working directory.
During the initial `daft up` command that you ran, the dependencies should have been specified in the configuration file.
During the cluster's initialization process, the cluster will download all the dependencies into a python virtual environment.
The working directory that you submit will then be run in that virtual environment, thus enabling it to access those pre-downloaded dependencies.

### Example

```bash
# submit a job using the default .daft.toml configuration file
daft submit -i my-keypair.pem -w my-working-director

# submit a job using the default .daft.toml configuration file
daft submit -c my-custom-config.toml -i my-keypair.pem -w my-working-director
```
