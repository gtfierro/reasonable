Getting Started
===============

Install
-------

**Python package** (pre-built wheels, no Rust toolchain required):

.. code-block:: bash

   pip install reasonable   # Python 3.9+

**CLI binary** (requires a Rust toolchain):

.. code-block:: bash

   cargo install reasonable-cli

   # or build from source after cloning:
   cargo build -p reasonable-cli --release
   # binary lands at: ./target/release/reasonable

Basic usage
-----------

Pass one or more Turtle or N3 files to the ``reasonable`` binary.
The reasoner loads all triples, runs OWL 2 RL materialisation, then
writes the result to ``output.ttl`` (override with ``-o``):

.. code-block:: console

   reasonable ontology.ttl data.ttl -o result.ttl

You can mix as many input files as needed.  The reasoner reads each file
in order and treats all triples as a single combined graph.

Python quickstart
-----------------

Load from files on disk:

.. code-block:: python

   import reasonable

   r = reasonable.PyReasoner()
   r.load_file("ontology.ttl")
   r.load_file("data.ttl")
   triples = r.reason()          # list of (subject, predicate, object) rdflib nodes
   print(len(triples))

Load from an existing rdflib graph:

.. code-block:: python

   import rdflib
   import reasonable

   g = rdflib.Graph()
   g.parse("ontology.ttl")
   g.parse("data.ttl")

   r = reasonable.PyReasoner()
   r.from_graph(g)
   triples = r.reason()

   # collect into a new graph
   result = rdflib.Graph()
   for triple in triples:
       result.add(triple)

Incremental reasoning
---------------------

After the first ``reason()`` call the reasoner tracks which triples it
has already processed.  Calling ``reason()`` again is incremental —
only newly added triples are processed.

Use ``update_graph()`` when your data changes.  It replaces the base
triples and automatically picks incremental or full re-materialisation:

.. code-block:: python

   r = reasonable.PyReasoner()
   r.from_graph(ontology + initial_data)
   r.reason()                         # full materialisation

   # data changes over time…
   new_data.add(triple_a)
   new_data.remove(triple_b)
   removed = r.update_graph(ontology + new_data)
   r.reason()                         # full re-mat when removed=True, else incremental

``update_graph()`` returns ``True`` when removals were detected (which
forces a full re-materialisation on the next ``reason()`` call) and
``False`` otherwise.

Building from source
--------------------

A Rust toolchain (via `rustup <https://rustup.rs/>`_) is required for
everything; `uv <https://docs.astral.sh/uv/>`_ and Python 3.9+ are
additionally needed for the Python bindings.

.. code-block:: bash

   make build               # release CLI binary
   make test                # Rust test suite
   make dev-python-library  # build Python extension into python/.venv
   make test-python         # build + run pytest
   make build-python-library  # distributable wheel → python/dist/

Building the docs
-----------------

.. code-block:: bash

   cd docs
   uv sync
   uv run sphinx-build -M html . _build
   open _build/html/index.html
