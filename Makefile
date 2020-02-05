.PHONY: test build

build:
	cargo build --release

test:
	cargo test

test-python:
	cargo build --lib --release --features "python-library"
	cp ./target/release/libreasonable.so reasonable.so
	python test.py

python-library:
	poetry run maturin build -b pyo3 --cargo-extra-args="--features python-library"
	poetry run maturin publish -b pyo3 --cargo-extra-args="--features python-library"

bench: build
	./scripts/bench_examples.sh
