from setuptools import setup
from setuptools.command.build_py import build_py
import subprocess

class CustomBuild(build_py):
    def run(self):
        from importlib import metadata
        print(metadata.version("asdf"))
        # subprocess.run(['python', 'scripts/build_default_tomls.py'], check=True)
        super().run()

setup(
    name='asdf',
    version='0.1.0',
    cmdclass={'build_py': CustomBuild},
)
