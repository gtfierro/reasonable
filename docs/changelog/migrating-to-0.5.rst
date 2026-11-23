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

The Python Debian package is now named ``python3-reasonable``. Update scripts
that install or locate the old ``pyreasonable`` package. The Rust crate is
still ``pyreasonable``, and Python code continues to use ``import reasonable``.
The CLI Debian package remains ``reasonable``.

If you build the Python Debian package, use ``make deb-python`` or
``cargo deb -p pyreasonable`` from the source checkout. The build now uses
``cargo-deb`` directly. Update scripts that invoke ``wheel2deb`` or
``make deb-python-cargo``.
