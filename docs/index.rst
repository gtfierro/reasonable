Reasonable
==========

An OWL 2 RL reasoner with reasonable performance.

Reasonable materialises the deductive closure of an RDF graph under the
`OWL 2 RL profile <https://www.w3.org/TR/owl2-profiles/#OWL_2_RL>`_.
It is implemented in Rust and exposed through a command-line tool and
Python bindings.

.. list-table::
   :widths: auto

   * - PyPI
     - ``pip install reasonable``
   * - Cargo (CLI)
     - ``cargo install reasonable-cli``
   * - GitHub
     - https://github.com/gtfierro/reasonable
   * - License
     - BSD-3-Clause

Performance
-----------

Benchmarked on Brick models of varying sizes, Reasonable is roughly 7×
faster than Allegro and 38× faster than OWLRL.

Quick start
-----------

**CLI** — load one or more Turtle/N3 files and write the materialised
output:

.. code-block:: console

   reasonable ontology.ttl data.ttl -o output.ttl

**Python** — reason over an rdflib graph:

.. code-block:: python

   import rdflib
   import reasonable

   g = rdflib.Graph()
   g.parse("ontology.ttl")
   g.parse("data.ttl")

   r = reasonable.PyReasoner()
   r.from_graph(g)
   triples = r.reason()

Contents
--------

.. toctree::
   :maxdepth: 2

   getting-started
   python-api/index
   cli/index
   owl-rules/index
   Rust API (docs.rs) <https://docs.rs/reasonable>
