#!/bin/sh
set -ex

cargo clean
cargo clippy  # A collection of lints to catch common mistakes and improve your Rust code.

cargo test --features disable_visualizer
cargo test --features disable_visualizer,u32_index
cargo test --features unsafe_pointer
cargo test --features unsafe_pointer,disable_visualizer
cargo test

cargo run --release -- test serial
cargo run --release -- test dual-parallel
cargo run --release -- test parallel

# just test one case would be enough
cargo run --release --features u32_index -- test serial

# test memory safety for unsafe implementations
cargo run --release --features unsafe_pointer -- test parallel
cargo run --release --features unsafe_arc -- test parallel
