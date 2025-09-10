# reasonable (Python bindings)

Python bindings for Reasonable — a reasonably fast OWL 2 RL reasoner implemented in Rust. This package exposes a small, typed API over rdflib terms to run materialization on RDF graphs or files.

## Quick Usage
Load from files (Turtle/N3) and materialize inferred triples:

```python
import reasonable

r = reasonable.PyReasoner()
r.load_file("../example_models/ontologies/Brick.n3")
r.load_file("../example_models/small1.n3")
out = r.reason()  # list[tuple[rdflib.term.Node, Node, Node]]
print(len(out))
```

Reason directly over an rdflib `Graph`:

```python
import rdflib
import reasonable
from rdflib.term import URIRef

g = rdflib.Graph()
g.add((URIRef("urn:s"), URIRef("urn:p"), URIRef("urn:o")))

r = reasonable.PyReasoner()
r.from_graph(g)
triples = r.reason()

# Optionally collect into a new Graph
g_out = rdflib.Graph()
for s, p, o in triples:
    g_out.add((s, p, o))
print(len(g_out))
```

## Install
- Runtime dependency: `rdflib`
- If you have a prebuilt wheel: `pip install dist/reasonable-*.whl`
- Build from source (see below) if no wheel is available for your platform.

## Developer Install (from source)
Using uv (recommended for local dev):

```bash
cd python
# Install project dependencies (including dev tools) into a managed venv
uv sync --group dev

# Build and develop-install the extension module
uv run maturin develop -b pyo3 --release

# Sanity check
uv run python -c "import reasonable; print(reasonable.__version__)"
```

Without uv (system/venv):

```bash
cd python
python -m venv .venv && . .venv/bin/activate  # or use your env
pip install -U maturin
maturin develop -b pyo3 --release
python -c "import reasonable; print(reasonable.__version__)"
```

## API Reference (minimal)
- `reasonable.PyReasoner()`
  - `load_file(path: str) -> None`
    - Load triples from a Turtle or N3 file. Raises `OSError` on missing/invalid paths.
  - `from_graph(graph_or_iterable) -> None`
    - Accepts an rdflib `Graph` or any iterable of 3-tuples convertible to rdflib-like nodes.
  - `reason() -> list[tuple[Node, Node, Node]]`
    - Runs OWL 2 RL materialization and returns all known triples as rdflib nodes.

## Building Wheels
Build release wheels into `dist/`:

```bash
cd python
uv run maturin build --release --out dist
```

Install the built wheel:

```bash
pip install dist/reasonable-*.whl
```

## Requirements
- Python 3.9+ (ABI3, built with `pyo3/abi3-py39`)
- Rust toolchain (`rustup`, `cargo`) for local builds
- `maturin` for building wheels
- `rdflib` (runtime dependency)

## Testing
Run the Python test suite (uses `pytest` and rdflib):

```bash
cd python
uv run pytest -q
```

Alternatively, with a local venv:

```bash
cd python
pip install -U maturin pytest rdflib
maturin develop -b pyo3 --release
pytest -q
```

## Compatibility Notes
- Python: 3.9+ is required due to the ABI3 setting in the Rust crate (`abi3-py39`).
- Platforms: macOS, Linux, and Windows are supported by PyO3/maturin; building from source requires a Rust toolchain.

## Troubleshooting
- `ModuleNotFoundError: No module named 'reasonable'`:
  - Ensure you ran `maturin develop` in the same environment you’re importing from.
- Build/link errors on macOS (Xcode/SDK):
  - Install Command Line Tools: `xcode-select --install`.
- `OSError` when calling `load_file(...)`:
  - Check the path and file format (Turtle/N3). Use absolute paths when in doubt.

## Contributing (Python bindings)
- Keep tests under `python/tests/` minimal and representative. Prefer inputs from `example_models/`.
- Format/lint Python with standard tooling; Rust code follows `cargo fmt`/`cargo clippy`.
- For broader project info (CLI, library, benchmarks), see the repository root `README.md`.
