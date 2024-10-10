## Dashboard

The dashboard command enables you to view the Ray dashboard of a specified cluster that you currently have running.
The way this is done is by establishing a port-forward over SSH from your local machine to the head node of the cluster (connecting `localhost:8265` to the remote head's `8265`).
The head node then serves some HTML/CSS/JS that you can view in your browser by pointing it towards `localhost:8265`).

An important thing to note is that this command will require that you have the appropriate private SSH keypair to authenticate against the remote head's public SSH keypair.
You will need to pass this SSH keypair as an argument to the command.

### Example

```bash
# establish the port-forward using the default .daft.toml configuration file
daft dashboard -i my-keypair.pem

# or, if you want, establish the port-forward using a custom configuration file
daft dashboard -c my-custom-config.toml -i my-keypair.pem
```
