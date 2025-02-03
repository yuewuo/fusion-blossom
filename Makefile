all: test check build python

fmt:
	cargo fmt --check

# A collection of lints to catch common mistakes and improve your Rust code.
clippy:
	cargo clippy -- -Dwarnings

clean:
	cargo clean

clean-env: clean fmt clippy

test: clean-env
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
	cargo run --release --features dangerous_pointer -- test parallel

build: clean-env
	cargo test --no-run --features u32_index
	cargo test --no-run --features u32_index --release
	cargo test --no-run --features disable_visualizer,u32_index --release
	cargo test --no-run --features qecp_integrate

	cargo test --no-run
	cargo test --no-run --release
	cargo test --no-run --features unsafe_pointer
	cargo test --no-run --features unsafe_pointer --release
	cargo test --no-run --features i32_weight
	cargo test --no-run --features i32_weight --release
	cargo test --no-run --features disable_visualizer
	cargo test --no-run --features disable_visualizer --release

	cargo build
	cargo build --release

check: clean-env
	cargo check --features u32_index
	cargo check --features u32_index --release
	cargo check --features disable_visualizer,u32_index --release
	cargo check --features qecp_integrate

	cargo check --release
	cargo check --features unsafe_pointer
	cargo check --features unsafe_pointer --release
	cargo check --features i32_weight
	cargo check --features i32_weight --release
	cargo check --features disable_visualizer
	cargo check --features disable_visualizer --release

wasm-check:
	cargo check --lib --no-default-features --features wasm_binding,remove_blossom_v
wasm:
	wasm-pack build --no-default-features --features wasm_binding,remove_blossom_v

python: clean-env
	maturin develop
	pytest tests/python

ci_rust_test:
	cargo test --release
	cargo test -r --no-default-features --features remove_blossom_v,dangerous_pointer,u32_index,i32_weight
