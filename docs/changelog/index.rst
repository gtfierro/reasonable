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
* Evaluates class unions and intersections from their declared RDF list
  heads, including when separate lists share a tail.
* Propagates class membership between compatible ``owl:someValuesFrom``
  restrictions (``scm-svf1``). This lets disjointness checks see types
  inferred through matching anonymous restrictions.
  See `issue #31 <https://github.com/gtfierro/reasonable/issues/31>`_.
* Python can read selected Pyoxigraph store quads directly and return the
  materialised closure as quads for a destination graph. See
  `issue #54 <https://github.com/gtfierro/reasonable/issues/54>`_.

See the :doc:`../owl-rules/index` for rule coverage and
:doc:`migrating-to-0.5` for upgrade steps.

.. toctree::
   :maxdepth: 1

   migrating-to-0.5
