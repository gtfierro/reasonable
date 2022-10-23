.PHONY: test build

#PY36_VERSION=3.6.10
PY37_VERSION=3.7.10
PY38_VERSION=3.8.8
PY39_VERSION=3.9.7
#PY3A_VERSION=3.10.0
#PY36_PATH=$(shell pyenv prefix $(PY36_VERSION))
PY37_PATH=$(shell pyenv prefix $(PY37_VERSION))
PY38_PATH=$(shell pyenv prefix $(PY38_VERSION))
PY39_PATH=$(shell pyenv prefix $(PY39_VERSION))
#PY3A_PATH=$(shell pyenv prefix $(PY3A_VERSION))
PY36_BIN=$(PY36_PATH)/bin/python
PY37_BIN=$(PY37_PATH)/bin/python
PY38_BIN=$(PY38_PATH)/bin/python
PY39_BIN=$(PY39_PATH)/bin/python
PY3A_BIN=$(PY3A_PATH)/bin/python

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
	#pyenv install -s $(PY36_VERSION)
	pyenv install -s $(PY37_VERSION)
	pyenv install -s $(PY38_VERSION)
	pyenv install -s $(PY39_VERSION)
	#pyenv install -s $(PY3A_VERSION)

python-build-remove:
	# cargo build --lib --release -v --features python-library -Zunstable-options -- --pretty=expanded
	cargo rustc --lib --release  --features python-library --profile=check -- -Zunstable-options --pretty=expanded

dev-python-library:
	cd python && poetry run maturin develop -b pyo3 --release
	poetry run python lang.py

build-python-library:
	cd python && poetry run maturin build  -b pyo3

bench: build
	./scripts/bench_examples.sh
