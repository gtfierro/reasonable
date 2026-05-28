Reasonable
==========

.. image:: https://img.shields.io/pypi/v/reasonable.svg
   :target: https://pypi.org/project/reasonable/
   :alt: PyPI version

.. image:: https://img.shields.io/crates/v/reasonable.svg
   :target: https://crates.io/crates/reasonable
   :alt: Crates.io version for reasonable

.. image:: https://img.shields.io/crates/v/reasonable-cli.svg
   :target: https://crates.io/crates/reasonable-cli
   :alt: Crates.io version for reasonable-cli

.. image:: https://img.shields.io/docsrs/reasonable.svg
   :target: https://docs.rs/reasonable
   :alt: docs.rs

.. image:: https://img.shields.io/badge/license-BSD--3--Clause-blue.svg
   :target: https://github.com/gtfierro/reasonable/blob/master/LICENSE
   :alt: BSD-3-Clause license

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
