# Notes

## Python Binding Development

```sh
maturin develop
python3 scripts/demo.py

# build for `manylinux`: widely usable wheels for linux pypi
docker run --rm -v $(pwd):/io ghcr.io/pyo3/maturin build --release  # or other maturin arguments
maturin build
maturin publish
```

## GitHub Build Wheels

To test GitHub CI locally on Ubuntu:

```sh
# https://github.com/nektos/act
act --container-architecture linux/amd64
```

test cibuildwheel locally on MacOS:

```sh
CIBW_BUILD=cp39-* CIBW_PLATFORM=macos CIBW_ARCHS_MACOS=universal2 python -m cibuildwheel --output-dir wheelhouse
```
