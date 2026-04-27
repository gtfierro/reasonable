CLI Reference
=============

The ``reasonable`` binary loads one or more RDF files, runs OWL 2 RL
materialisation, and writes the result to disk.

Install
-------

.. code-block:: bash

   cargo install reasonable-cli

   # or build from the workspace after cloning:
   cargo build -p reasonable-cli --release
   ./target/release/reasonable --help

Usage
-----

.. code-block:: text

   reasonable [OPTIONS] <INPUT_FILES>...

   Arguments:
     <INPUT_FILES>...   One or more Turtle or N3 input files

   Options:
     -o, --output-file <FILE>   Output file [default: output.ttl]
         --error-format <FMT>   Diagnostic output format: text | json | ndjson [default: text]
         --fail-on <CODES>      Exit with code 2 if any of these rule codes occur
         --max-diagnostics <N>  Limit the number of diagnostics printed
         --summary-only         Print only a count of diagnostics, not individual messages
     -h, --help                 Print help
     -V, --version              Print version

Basic example
-------------

.. code-block:: console

   reasonable Brick.n3 building_model.n3 -o result.ttl

All input files are merged into a single graph before reasoning begins.

Diagnostics
-----------

The reasoner records OWL 2 RL rule violations as *diagnostics* rather
than aborting.  By default they are printed to stdout in plain text
after the output file is written.

Change the format with ``--error-format``:

.. code-block:: console

   # machine-readable JSON array
   reasonable model.ttl --error-format json -o out.ttl

   # one JSON object per line (newline-delimited)
   reasonable model.ttl --error-format ndjson -o out.ttl

Each diagnostic has four fields: ``code``, ``rule``, ``severity``, and
``message``.

Failing on specific violations
-------------------------------

Use ``--fail-on`` to exit with code **2** when a particular rule
violation is detected.  Pass rule codes as a comma-separated list or
repeat the flag:

.. code-block:: console

   reasonable model.ttl --fail-on cax-dw,prp-pdw -o out.ttl

   # codes are case-insensitive; OWLRL.* prefix is also accepted
   reasonable model.ttl --fail-on OWLRL.CAX_DW -o out.ttl

Common diagnostic codes
-----------------------

.. list-table::
   :header-rows: 1
   :widths: 25 75

   * - Code
     - Meaning
   * - ``OWLRL.CAX_DW``
     - Individual typed as two disjoint classes
   * - ``OWLRL.PRP_PDW``
     - Pair of individuals violates ``owl:propertyDisjointWith``
   * - ``OWLRL.PRP_ASYP``
     - Asymmetric property asserted in both directions
   * - ``OWLRL.PRP_IRP``
     - Irreflexive property used with the same subject and object
   * - ``OWLRL.CLS_NOTHING``
     - Individual typed as ``owl:Nothing``

Limiting output
---------------

.. code-block:: console

   # print at most 10 diagnostics
   reasonable model.ttl --max-diagnostics 10 -o out.ttl

   # print only the total count, not individual messages
   reasonable model.ttl --summary-only -o out.ttl

Environment
-----------

Set ``RUST_LOG`` to control log verbosity:

.. code-block:: console

   RUST_LOG=debug reasonable model.ttl -o out.ttl

The default level is ``info``, which logs file load times and the total
reasoning duration.
