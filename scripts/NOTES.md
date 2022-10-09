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
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
CIBW_BUILD=cp39-* CIBW_PLATFORM=macos CIBW_ARCHS_MACOS=universal2 python -m cibuildwheel --output-dir wheelhouse
```

To manually upload the wheels

```sh
twine upload artifact-0.1.0/* --repository-url https://upload.pypi.org/legacy/
```

## Jenkins CI

I need to manually download Blossom V library

```sh
sudo -sH -u jenkins
cd /var/lib/jenkins/workspace

cd FusionBlossomBuildDev
wget -c https://pub.ist.ac.at/~vnk/software/blossom5-v2.05.src.tar.gz -O - | tar -xz
cp -r blossom5-v2.05.src/* blossomV/
rm -r blossom5-v2.05.src
```
