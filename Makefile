.PHONY: build build-tiny test bench dev-python-library build-python-library test-python bench-python deb deb-python

build:
	cargo build --release

build-tiny:
	RUSTFLAGS='-C link-arg=-s' cargo build --release

test:
	cargo test

bench: build
	./scripts/bench_examples.sh

dev-python-library:
	cd python && uv run maturin develop -b pyo3 --release

build-python-library:
	cd python && uv run maturin build -b pyo3 --release --out dist

test-python: dev-python-library
	cd python && uv run pytest -q

bench-python:
	cd benches/python && uv run bench.py

deb:
	# Requires: cargo install cargo-deb
	./scripts/build_deb.sh

deb-python:
	./scripts/build_python_deb.sh
