# Example

Okay, let's try our hand with an example project.
Let's spin up a cluster and submit a basic job to execute on it.

This project will proceed assuming you're using `uv` and `aws`.
However, the concepts should translate to whatever python package manager and cloud provider that you specifically choose.

## Prerequisites

The following should be installed on your machine:
- The [aws cli tool](https://aws.amazon.com/cli).
  - Assuming you're using aws as your cloud provider.
- Some type of python package manager.
We recommend using [`uv`](https://docs.astral.sh/uv) to manage everything (i.e., dependencies, as well as the python version itself).

## Permissions
...

## Getting started

```bash
# create the project directory
cd some/working/directory
mkdir launch-test
cd launch-test

# initialize the project
uv init --python 3.12
uv venv
source .venv/bin/activate

# install daft-launcher
uv pip install "daft-launcher"
```

So at this point, you should have a working python project.
You should also have the actual daft-launcher CLI tool installed as well.
In fact, if you run `daft --version`, you should see the version of the CLI tool printed to the screen.
You can even try running `daft --help`; you should get some helpful output printed!

Other commands for daft-launcher, however, may still not work just yet.
This is because you may not have configured your aws credentials just yet.

In order to do so, you can run the following:

```bash
aws configure sso
```
