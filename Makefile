prepare:
	rustup default nightly-2022-05-19
	rustup target add wasm32-unknown-unknown

rust-test-only:
	cargo test -p tests

copy-wasm-file-to-test:
	cp target/wasm32-unknown-unknown/release/*.wasm tests/wasm

test: build-contract copy-wasm-file-to-test rust-test-only

clippy:
	cargo clippy --all-targets --all -- -D warnings

check-lint: clippy
	cargo fmt --all -- --check

format:
	cargo fmt --all

lint: clippy format

build-contract:
	cargo build --release -p bid-purse --target wasm32-unknown-unknown
	cargo build --release -p dutch-auction-installer --target wasm32-unknown-unknown
	cargo build --release -p english-auction-installer --target wasm32-unknown-unknown
	cargo build --release -p swap-installer --target wasm32-unknown-unknown
	cargo build --release -p gift-installer --target wasm32-unknown-unknown
	wasm-strip target/wasm32-unknown-unknown/release/dutch-auction-installer.wasm
	wasm-strip target/wasm32-unknown-unknown/release/english-auction-installer.wasm
	wasm-strip target/wasm32-unknown-unknown/release/swap-installer.wasm
	wasm-strip target/wasm32-unknown-unknown/release/bid-purse.wasm
	wasm-strip target/wasm32-unknown-unknown/release/extend-bid-purse.wasm
	wasm-strip target/wasm32-unknown-unknown/release/delta-bid-purse.wasm
	wasm-strip target/wasm32-unknown-unknown/release/gift-installer.wasm

clean:
	cargo clean


