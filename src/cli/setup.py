from setuptools import setup, find_packages

setup(
    name="iluvatar_cli",
    version="1.0.0",
    packages=find_packages(),
    install_requires=[
        "click",
    ],
    entry_points={
        "console_scripts": [
            "iluvatar_cli=iluvatar_cli.cli:main",
        ],
    },
)
