.PHONY: test build

build:
	cargo build --release

test:
	cargo test

bench: build
	./scripts/bench_examples.sh
