# Future Plans

The following is a non-exhaustive list of ideas for future improvements to the daft launcher project:

## Real-time logs

During development, it is often useful to see the output of the job in real-time.
Currently, print out statements are not available, which can hinder quick debugging methods.
We want to extend the launcher such that instead of submitting the job and immediately closing the connection, the connection will remain open and the stdout of the head node will be printed out in real-time.
The connection will only be closed when the remote process is finished itself.

## Improved detection of local keypairs

Currently, you need to specify the path to the keypair in the configuration file.
This is not ideal.
The daft launcher should be able to detect the keypair automatically by querying the remote instance to pull the name of the public key, and using that public key name to find the locally stored private key.
This will allow for a more seamless experience when using the launcher.
In the case that you have changed the name of the private key locally, you can always fall back to manually specifying its path.
