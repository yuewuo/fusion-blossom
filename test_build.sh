#!/bin/sh
set -ex

cargo clean
cargo clippy  # A collection of lints to catch common mistakes and improve your Rust code.

# check this first because it's easy to have errors
cargo build --features u32_index
cargo build --features u32_index --release

cargo build
cargo build --release
cargo build --features unsafe_pointer
cargo build --features unsafe_pointer --release
cargo build --features i32_weight
cargo build --features i32_weight --release
cargo build --features disable_visualizer
cargo build --features disable_visualizer --release
# cargo build --features python_binding
# cargo build --features python_binding --release
