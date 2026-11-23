Migrating to 0.5
================

.. note::

   This draft describes changes in the 0.5 release series for users upgrading
   from 0.4.4. The stable 0.5 release is still in preparation.

Review changes to inferred triples
----------------------------------

Property-chain reasoning (``prp-spo2``) is enabled by default in the Rust
library, CLI and Python bindings. When an ontology declares an
``owl:propertyChainAxiom``, the reasoner can infer additional triples from
paths that match the chain. Regenerate expected output for tests that compare
the entire materialised graph, and review downstream queries that depend on
which relationships are inferred. See :doc:`../owl-rules/index`.

Rust applications that need to disable property chains can set
``default-features = false`` on their ``reasonable`` dependency. To build
the CLI without property chains from a source checkout, run
``cargo build -p reasonable-cli --release --no-default-features``. To build
an ABI3 wheel without property chains, run
``maturin build --release --features abi3 --no-default-features`` from the
``python`` directory.

The fix for inverse-functional properties limits ``owl:sameAs`` inference to
subjects that share an object through the same inverse-functional property.
If you persisted graphs materialised with an affected version, rebuild them
from the original asserted triples and replace the old inferred output.
This also removes triples derived from the incorrect equivalences.
See `issue #64 <https://github.com/gtfierro/reasonable/issues/64>`_.

Update Debian packaging scripts
-------------------------------

Install the Python Debian package as ``python3-reasonable``, the same name
used by the `0.4.4 release assets <https://github.com/gtfierro/reasonable/releases/expanded_assets/v0.4.4>`_.
The Rust crate is ``pyreasonable``, and Python code uses ``import reasonable``.

The build now uses ``cargo-deb``. Use ``make deb-python`` or
``cargo deb -p pyreasonable`` from the source checkout in place of direct
``wheel2deb`` invocations. Update artifact paths from
``dist/wheel2deb/output/`` to ``target/debian/`` for local builds. Scripts
that fetch release assets should look for the new ABI3 package filenames,
which use the form ``python3-reasonable_<version>_<arch>.deb``.
