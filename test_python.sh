#!/bin/sh
set -ex

# install latest version
maturin develop

pytest tests/python
