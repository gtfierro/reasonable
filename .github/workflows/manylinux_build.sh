cd /workdir
yum -y install centos-release-scl-rh
yum -y install llvm-toolset-7.0
source scl_source enable llvm-toolset-7.0
curl https://sh.rustup.rs -sSf | sh -s -- -y
export PATH="${PATH}:/root/.cargo/bin:/opt/python/cp37-cp37m/bin:/opt/python/cp38-cp38/bin:/opt/python/cp39-cp39/bin:/opt/python/cp310-cp310/bin:/opt/python/cp311-cp311/bin"
curl -sSL https://install.python-poetry.org | python -
poetry install
poetry run pip install poetry
poetry run maturin build --release -m Cargo.toml -F python
poetry run python -c "import reasonable"
