Python API
==========

The ``reasonable`` Python package exposes the Rust reasoner through
`PyO3 <https://pyo3.rs>`_ bindings.  Pre-built wheels are on PyPI for
Python 3.9+ on macOS, Linux, and Windows.

.. code-block:: bash

   pip install reasonable

The main class is ``PyReasoner``.  Create one instance per reasoning
session; it accumulates base triples and manages incremental state.

.. code-block:: python

   import reasonable
   r = reasonable.PyReasoner()

PyReasoner
----------

``PyReasoner()``
   Create a new reasoner instance with an empty triple store.

``load_file(path: str) -> None``
   Append all triples from a Turtle or N3 file at *path* to the base
   graph.  Raises ``OSError`` if the file is missing or cannot be
   parsed.  Call multiple times to load several files.

``from_graph(graph_or_iterable) -> None``
   Append triples from an ``rdflib.Graph`` or any iterable of
   ``(subject, predicate, object)`` 3-tuples.  Use ``update_graph()``
   instead when you need retraction support.

``update_graph(graph_or_iterable) -> bool``
   Replace the base triples with the contents of the given graph.
   Computes a diff against the current base:

   - If only additions are found, the next ``reason()`` uses
     incremental materialisation.
   - If any removals are detected, the next ``reason()`` performs a
     full re-materialisation.

   Returns ``True`` when removals were detected.

``reason() -> list[tuple[Node, Node, Node]]``
   Run OWL 2 RL materialisation and return all known triples (base
   plus inferred) as rdflib nodes.  After the first call, subsequent
   calls are incremental unless removals were detected via
   ``update_graph()``.

``reason_full() -> list[tuple[Node, Node, Node]]``
   Force a full re-materialisation from base triples, ignoring any
   incremental state.  Equivalent to ``clear()`` followed by
   ``reason()``.

``clear() -> None``
   Reset all inferred state while keeping base triples.  The next
   ``reason()`` call will perform a full re-materialisation.

``get_base_triples() -> list[tuple[Node, Node, Node]]``
   Return the current base (non-inferred) triples as rdflib nodes.
   Useful for debugging.

Examples
--------

Collect inferred triples into a new graph:

.. code-block:: python

   import rdflib
   import reasonable

   g = rdflib.Graph()
   g.parse("ontology.ttl")
   g.parse("data.ttl")

   r = reasonable.PyReasoner()
   r.from_graph(g)

   result = rdflib.Graph()
   for triple in r.reason():
       result.add(triple)

   print(f"{len(result)} triples after materialisation")

Incremental update with retraction:

.. code-block:: python

   import rdflib
   import reasonable
   from rdflib import URIRef, RDF

   ontology = rdflib.Graph()
   ontology.parse("ontology.ttl")

   data = rdflib.Graph()
   data.add((URIRef("urn:sensor1"), RDF.type, URIRef("urn:TemperatureSensor")))

   r = reasonable.PyReasoner()
   r.from_graph(ontology + data)
   r.reason()

   # data changes…
   data.remove((URIRef("urn:sensor1"), RDF.type, URIRef("urn:TemperatureSensor")))
   data.add((URIRef("urn:sensor2"), RDF.type, URIRef("urn:HumiditySensor")))

   r.update_graph(ontology + data)
   triples = r.reason()

.. note::

   ``from_graph()`` *appends* to existing base triples.  If you want
   to replace the base entirely, use ``update_graph()`` instead.

Building from source
--------------------

.. code-block:: bash

   cd python
   uv sync --group dev
   uv run maturin develop -b pyo3 --release

   # verify
   uv run python -c "import reasonable; print(reasonable.__version__)"

Run the test suite:

.. code-block:: bash

   cd python
   uv run pytest -q
