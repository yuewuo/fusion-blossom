#!/bin/sh
set -ex

cargo clippy  # A collection of lints to catch common mistakes and improve your Rust code.

# check this first because it's easy to have errors
cargo check --features u32_index
cargo check --features u32_index --release
cargo check --features disable_visualizer,u32_index --release

cargo check --release
cargo check --features unsafe_pointer
cargo check --features unsafe_pointer --release
cargo check --features i32_weight
cargo check --features i32_weight --release
cargo check --features disable_visualizer
cargo check --features disable_visualizer --release
# cargo check --features python_binding
# cargo check --features python_binding --release
