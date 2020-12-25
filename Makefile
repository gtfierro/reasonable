.PHONY: test build

PY36_VERSION=3.6.10
PY37_VERSION=3.7.6
PY38_VERSION=3.8.2
PY38_VERSION=3.9.0
PY36_PATH=$(shell pyenv prefix $(PY36_VERSION))
PY37_PATH=$(shell pyenv prefix $(PY37_VERSION))
PY38_PATH=$(shell pyenv prefix $(PY38_VERSION))
PY39_PATH=$(shell pyenv prefix $(PY39_VERSION))
PY36_BIN=$(PY36_PATH)/bin/python
PY37_BIN=$(PY37_PATH)/bin/python
PY38_BIN=$(PY38_PATH)/bin/python
PY39_BIN=$(PY39_PATH)/bin/python

build:
	cargo build --release

build-tiny:
	RUSTFLAGS='-C link-arg=-s' cargo build --release

test:
	cargo test

# for macos, do
# cp target/release/libreasonable.dylib reasonable/reasonable.so
# this test is missing brick_inference_test.n3 so it doens't work 
test-python:
	cargo build --lib --release --features "python-library"
	cp ./target/release/libreasonable.so reasonable/reasonable.so
	python test.py

install-python-versions:
	# requires pyenv
	pyenv install -s $(PY36_VERSION)
	pyenv install -s $(PY37_VERSION)
	pyenv install -s $(PY38_VERSION)
	pyenv install -s $(PY39_VERSION)

build-python-library:
	poetry run maturin build -i $(PY36_BIN) -i $(PY37_BIN) -i $(PY38_BIN) -i $(PY39_BIN) -b pyo3 --cargo-extra-args="--features python-library"

publish-python-library: build-python-library
	poetry run maturin publish -i $(PY36_BIN) -i $(PY37_BIN) -i $(PY38_BIN) -i $(PY39_BIN) -b pyo3 --cargo-extra-args="--features python-library"

bench: build
	./scripts/bench_examples.sh
