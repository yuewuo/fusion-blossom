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

To test GitHub CI locally:

```sh
# https://github.com/nektos/act

```
