# Commands to Run:

- install uv
- install awscli

```bash
mkdir daft-launcher-test
cd daft-launcher-test

uv init --python 3.12
uv venv
source .venv/bin/activate
uv pip install "daft-launcher"
```

At this point, you need to create your AWS token (so that daft-launcher can use it to manage your clusters)

```bash
aws sso login
```

Now you are able to run `daft-launcher`

```bash
daft init-config
```

```bash
# list all the clusters
daft list

# spin a new cluster up
daft up

# submit a job to the cluster
daft submit \
    --identity-file ~/.ssh/MY_PUBLIC_KEY.pem \
    --working-dir /path/to/working/dir \
    -- python main.py

# spin that cluster down
daft down
```
