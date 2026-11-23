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

``diagnostics() -> list[dict[str, str]]``
   Return the collected reasoning diagnostics. Each dictionary contains
   ``code``, ``rule``, ``severity`` and ``message``. Call this after
   ``reason()`` or ``reason_full()`` to inspect violations such as
   ``OWLRL.CAX_ADC`` for an individual belonging to two classes in the same
   ``owl:AllDisjointClasses`` group. Use the ``code`` field for filtering.
   See :doc:`../owl-rules/index` for rule coverage and diagnostic codes.
   An empty list means that none of the implemented checks reported a
   violation.

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

Pyoxigraph stores
-----------------

Install the optional extra with ``pip install 'reasonable[pyoxigraph]'``.
The quad methods share the base triples and incremental state of
``PyReasoner``.

``add_quads(quads_or_triples) -> None``
   Append an iterable of Pyoxigraph ``Quad`` or ``Triple`` objects.

``update_quads(quads_or_triples) -> bool``
   Replace the base triples. Returns ``True`` if removals require full
   re-materialisation. Additions use incremental reasoning.

``reason_quads(graph_name=None) -> list[pyoxigraph.Quad]``
   Return the base and inferred triples as quads in the requested graph.
   The graph name may be a ``NamedNode``, ``BlankNode``, or
   ``DefaultGraph``. The default is ``DefaultGraph()``. Call
   ``diagnostics()`` after reasoning to inspect rule violations.

Each input call accepts quads from one graph and loads their triples into
the reasoner's single graph. Mixed graph names raise ``ValueError``.
To reason over a deliberate union of source graphs, pass an iterable of
``quad.triple`` values. RDF-star triple terms and RDF 1.2 directional
language-tagged literals raise ``TypeError``. Filter those terms before
loading a graph that contains them.

Refresh a materialised graph from the store's default graph:

.. code-block:: python

   from pyoxigraph import DefaultGraph, NamedNode, Store
   from reasonable import PyReasoner

   store = Store("path/to/store")
   reasoner = PyReasoner()
   target = NamedNode("urn:reasoned")

   def refresh():
       source = store.quads_for_pattern(None, None, None, DefaultGraph())
       reasoner.update_quads(source)
       closure = reasoner.reason_quads(graph_name=target)
       store.clear_graph(target)
       store.bulk_extend(closure)

   refresh()

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
