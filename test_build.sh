#!/bin/sh

cargo clean

# check this first because it's easy to have errors
cargo build --features u32_index || exit 1
cargo build --features u32_index --release || exit 1

cargo build || exit 1
cargo build --release || exit 1
cargo build --features unsafe_pointer || exit 1
cargo build --features unsafe_pointer --release || exit 1
cargo build --features i32_weight || exit 1
cargo build --features i32_weight --release || exit 1
cargo build --features disable_visualizer || exit 1
cargo build --features disable_visualizer --release || exit 1
cargo build --features python_binding || exit 1
cargo build --features python_binding --release || exit 1
