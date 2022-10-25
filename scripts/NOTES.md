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

To also upload the source package

```sh
maturin sdist
twine upload target/wheels/fusion_blossom-0.2.0.tar.gz --repository-url https://upload.pypi.org/legacy/
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


## Benchmarking and Profiling


See https://github.com/flamegraph-rs/flamegraph

enable the following debug configuration, and then run `cargo flamegraph -- ...` which is equivalent to `cargo run --release -- ...`;
and then visit: http://localhost:8066/partition-profile.html?filename=tmp/flamegraph-test.profile
example:

```shell
# generate data
cargo run --release -- benchmark 15 -r 20 -n 10000 0.005 --code-type phenomenological-planar-code --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"visualize/data/tmp/flamegraph-test.syndromes"}'
# 1 partitions
cargo flamegraph -o visualize/data/tmp/flamegraph-test.svg --root -- benchmark 15 -r 20 -n 10000 0.005 --code-type error-pattern-reader --code-config '{"filename":"visualize/data/tmp/flamegraph-test.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":1,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output visualize/data/tmp/flamegraph-test.profile
# 4 partitions
cargo flamegraph -o visualize/data/tmp/flamegraph-test.svg --root -- benchmark 15 -r 20 -n 10000 0.005 --code-type error-pattern-reader --code-config '{"filename":"visualize/data/tmp/flamegraph-test.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":4,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output visualize/data/tmp/flamegraph-test.profile
# 100 partitions
cargo flamegraph -o visualize/data/tmp/flamegraph-test.svg --root -- benchmark 15 -r 20 -n 10000 0.005 --code-type error-pattern-reader --code-config '{"filename":"visualize/data/tmp/flamegraph-test.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":100,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output visualize/data/tmp/flamegraph-test.profile
# 1000 partitions
cargo flamegraph -o visualize/data/tmp/flamegraph-test.svg --root -- benchmark 15 -r 20 -n 10000 0.005 --code-type error-pattern-reader --code-config '{"filename":"visualize/data/tmp/flamegraph-test.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":1000,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output visualize/data/tmp/flamegraph-test.profile
```

```shell
# default feature
cargo flamegraph -o visualize/data/tmp/flamegraph-test.svg --root -- benchmark 15 -r 20 -n 10000 0.005 --code-type error-pattern-reader --code-config '{"filename":"visualize/data/tmp/flamegraph-test.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":1,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output visualize/data/tmp/flamegraph-test.profile
# unsafe_pointer
cargo flamegraph -o visualize/data/tmp/flamegraph-test-unsafe-pointer.svg --features unsafe_pointer --root -- benchmark 15 -r 20 -n 10000 0.005 --code-type error-pattern-reader --code-config '{"filename":"visualize/data/tmp/flamegraph-test.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":1,"enable_tree_fusion":true}' --verifier none
# dangerous_pointer
cargo flamegraph -o visualize/data/tmp/flamegraph-test-unsafe-arc.svg --features dangerous_pointer --root -- benchmark 15 -r 20 -n 10000 0.005 --code-type error-pattern-reader --code-config '{"filename":"visualize/data/tmp/flamegraph-test.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":1,"enable_tree_fusion":true}' --verifier none
```
