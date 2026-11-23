Changelog
=========

Published versions and their release notes are listed on the
`GitHub releases page <https://github.com/gtfierro/reasonable/releases>`_.

0.5 (in preparation)
--------------------

* Materialises transitive subproperty links (``scm-spo``), subproperty
  links in both directions for equivalent properties (``scm-eqp1``), and
  class membership for individuals listed in ``owl:oneOf`` (``cls-oo``).
* Reports equality contradictions (``eq-diff1``), violations of
  ``owl:AllDifferent`` (``eq-diff2``, ``eq-diff3``), and violations of
  ``owl:AllDisjointClasses`` and ``owl:AllDisjointProperties``
  (``cax-adc`` and ``prp-adp``).

See the :doc:`../owl-rules/index` for rule coverage and
:doc:`migrating-to-0.5` for upgrade steps.

.. toctree::
   :maxdepth: 1

   migrating-to-0.5
