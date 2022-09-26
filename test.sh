#!/bin/sh

cargo clippy  # A collection of lints to catch common mistakes and improve your Rust code.

cargo test || exit 1
cargo test --features disable_visualizer || exit 1

cargo run --release -- test serial || exit 1
cargo run --release -- test dual-parallel || exit 1
cargo run --release -- test parallel || exit 1
