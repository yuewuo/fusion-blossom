#!/bin/sh

cargo clean
cargo clippy  # A collection of lints to catch common mistakes and improve your Rust code.

cargo test --features disable_visualizer || exit 1
cargo test --features disable_visualizer,u32_index || exit 1
cargo test || exit 1

cargo run --release -- test serial || exit 1
cargo run --release -- test dual-parallel || exit 1
cargo run --release -- test parallel || exit 1

# just test one case would be enough
cargo run --release --features u32_index -- test serial || exit 1
