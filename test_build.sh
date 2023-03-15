#!/bin/sh
set -ex

cargo clean
cargo clippy  # A collection of lints to catch common mistakes and improve your Rust code.

# check this first because it's easy to have errors
cargo test --no-run --features u32_index
cargo test --no-run --features u32_index --release
cargo test --no-run --features disable_visualizer,u32_index --release

cargo test --no-run
cargo test --no-run --release
cargo test --no-run --features unsafe_pointer
cargo test --no-run --features unsafe_pointer --release
cargo test --no-run --features i32_weight
cargo test --no-run --features i32_weight --release
cargo test --no-run --features disable_visualizer
cargo test --no-run --features disable_visualizer --release
# cargo test --no-run --features python_binding
# cargo test --no-run --features python_binding --release
