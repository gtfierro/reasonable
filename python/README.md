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

### Incremental Reasoning with Retraction

Use `update_graph()` to replace the reasoner's base triples when your graph changes. The reasoner automatically computes a diff and selects incremental materialization (additions only) or full re-materialization (if removals detected):

```python
import rdflib
import reasonable
from rdflib import URIRef, RDF

ontology = rdflib.Graph()
ontology.parse("my_ontology.ttl")

data = rdflib.Graph()
data.add((URIRef("urn:sensor1"), RDF.type, URIRef("urn:TemperatureSensor")))

r = reasonable.PyReasoner()
r.from_graph(ontology + data)
triples = r.reason()  # full materialization

# Later, data changes...
data.remove((URIRef("urn:sensor1"), RDF.type, URIRef("urn:TemperatureSensor")))
data.add((URIRef("urn:sensor2"), RDF.type, URIRef("urn:HumiditySensor")))

r.update_graph(ontology + data)  # replaces base, auto-detects diff
triples = r.reason()             # full re-mat (removals detected)
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

## API Reference
- `reasonable.PyReasoner()`
  - `load_file(path: str) -> None`
    - Load triples from a Turtle or N3 file. **Appends** to existing base triples. Raises `OSError` on missing/invalid paths.
  - `from_graph(graph_or_iterable) -> None`
    - **Appends** triples from an rdflib `Graph` (or any iterable of 3-tuples) to the base. Use `update_graph()` instead if you need retraction support.
  - `update_graph(graph_or_iterable) -> bool`
    - **Replaces** the base triples with the contents of the given graph. Computes a diff against the current base: if only additions are found, the next `reason()` uses incremental materialization; if any removals are detected, it triggers a full re-materialization. Returns `True` if removals were detected, `False` otherwise.
  - `reason() -> list[tuple[Node, Node, Node]]`
    - Runs OWL 2 RL materialization and returns all known triples (base + inferred) as rdflib nodes. After the first call, subsequent calls are incremental (only processing newly added triples) unless removals were detected via `update_graph()`.
  - `reason_full() -> list[tuple[Node, Node, Node]]`
    - Forces a full re-materialization from base triples, ignoring any incremental state. Equivalent to `clear()` followed by `reason()`.
  - `clear() -> None`
    - Resets all inferred state while keeping base triples. The next `reason()` call will perform a full re-materialization.
  - `get_base_triples() -> list[tuple[Node, Node, Node]]`
    - Returns the current base (non-inferred) triples as rdflib nodes. Useful for debugging.

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
