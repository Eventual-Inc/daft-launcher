# Future Plans

The following is a non-exhaustive list of ideas for future improvements to the daft launcher project:

- Enable real-time logs for jobs.
  Print out statements are currently not available, which can negatively affect development.
  Daft launcher is planning on being extended such that `daft submit ...` instead of submitting the job and then closing the connection, will keep the connection open and print out the stdout of the head node.
- Improved detection of local keypairs.
  Currently, you need to specify the path to the keypair in the configuration file.
  This is not ideal.
  The daft launcher should be able to detect the keypair automatically by querying the remote instance to pull the name of the public key, and using that public key name to find the locally stored private key.
