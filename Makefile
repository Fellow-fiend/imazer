.PHONY: build release run check fmt clean

build:
	cargo build

release:
	cargo build --release

run:
	cargo run

check:
	cargo check

fmt:
	cargo fmt

clean:
	cargo clean
