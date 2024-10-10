# Example

Okay, let's try our hand with an example project.
Let's spin up a cluster and submit a basic job to execute on it.

This project will proceed assuming you're using `uv` and `aws`.
However, the concepts should translate to whatever python package manager and cloud provider that you specifically choose.

## Prerequisites

The following should be installed on your machine:
- The [aws cli tool](https://aws.amazon.com/cli).
  (Assuming you're using aws as your cloud provider).
- Some type of python package manager.
  We recommend using [`uv`](https://docs.astral.sh/uv) to manage everything (i.e., dependencies, as well as the python version itself).
  It's much cleaner and faster than `pip`.

## Permissions

...

## Getting started

Run the following commands to initialize your project:

```bash
# create the project directory
cd some/working/directory
mkdir launch-test
cd launch-test

# initialize the project
uv init --python 3.12
uv venv
source .venv/bin/activate

# install daft launcher
uv pip install "daft-launcher"
```

At this point, you'll have a properly set up python project.
You'll have a pretty basic working directory.
It should look something like this:

```txt
/
|- .venv/
|- hello.py
|- pyproject.toml
|- README.md
|- .python-version
```

In your virtual environment, you'll have daft launcher installed.
You can verify this by running `daft --version`, which should return the latest version of daft launcher which is available.
You can even try running `daft --help` and see what commands are available.

Note that other commands for daft launcher may still not work just yet.
This is because most likely because you haven't configured your AWS credentials.
There are a couple of different ways of doing so, but for the purposes of this example, let's establish an SSO connection and verify that.
Thus, run the following:

```bash
# configure your sso
aws configure sso

# login to your sso
aws sso login
```

This should open up your browser.
Accept the following requests, and return to your terminal.
You see a success message from the aws cli tool.
At this point, your aws cli tool has been configured, and your environment is fully setup.

## Running a job

First, let's just get some boilerplate code out of the way.
Let's create a working directory and move our `hello.py` file into it.

```bash
mkdir src
mv hello.py src
```

Next, let's import daft and run a simple query inside of `hello.py`.

```python
import daft
df = daft.from_pydict({ "values": [0, 1, 8] })
df.with_column("result", daft.col("values").cbrt()).show()
```

Okay, now that we have some basic boilerplate code, let's actually try and run it using daft launcher.
