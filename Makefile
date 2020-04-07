.PHONY: test build

PY36_VERSION=3.6.10
PY37_VERSION=3.7.6
PY38_VERSION=3.8.2
PY36_PATH=$(shell pyenv prefix $(PY36_VERSION))
PY37_PATH=$(shell pyenv prefix $(PY37_VERSION))
PY38_PATH=$(shell pyenv prefix $(PY38_VERSION))
PY36_BIN=$(PY36_PATH)/bin/python
PY37_BIN=$(PY37_PATH)/bin/python
PY38_BIN=$(PY38_PATH)/bin/python

build:
	cargo build --release

test:
	cargo test

test-python:
	cargo build --lib --release --features "python-library"
	cp ./target/release/libreasonable.so reasonable.so
	python test.py

install-python-versions:
	# requires pyenv
	pyenv install $(PY36_VERSION)
	pyenv install $(PY37_VERSION)
	pyenv install $(PY37_VERSION)

python-library:
	poetry run maturin build -i $(PY36_BIN) -i $(PY37_BIN) -i $(PY38_BIN) -b pyo3 --cargo-extra-args="--features python-library"
	poetry run maturin publish -i $(PY36_BIN) -i $(PY37_BIN) -i $(PY38_BIN) -b pyo3 --cargo-extra-args="--features python-library"

bench: build
	./scripts/bench_examples.sh
