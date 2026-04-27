OWL 2 RL Rules
==============

Reasonable implements a subset of the
`OWL 2 RL reasoning rules <https://www.w3.org/TR/owl2-profiles/#Reasoning_in_OWL_2_RL_and_RDF_Graphs_using_Rules>`_
defined by the W3C.  The tables below show the current implementation
status.

Rules marked *throws exception* produce a diagnostic (see
:doc:`../cli/index`) rather than aborting reasoning.

RDFS Semantics
--------------

.. list-table::
   :header-rows: 1
   :widths: 15 20 65

   * - Done
     - Rule
     - Notes
   * - yes
     - ``rdfs11``
     - ``rdfs:subClassOf`` transitivity

Equality Semantics
------------------

.. list-table::
   :header-rows: 1
   :widths: 15 20 65

   * - Done
     - Rule
     - Notes
   * - no
     - ``eq-ref``
     - Not implemented — very inefficient; causes runaway flux
   * - yes
     - ``eq-sym``
     -
   * - yes
     - ``eq-trans``
     -
   * - yes
     - ``eq-rep-s``
     -
   * - yes
     - ``eq-rep-p``
     -
   * - yes
     - ``eq-rep-o``
     -
   * - no
     - ``eq-diff1``
     - Throws exception
   * - no
     - ``eq-diff2``
     - Throws exception
   * - no
     - ``eq-diff3``
     - Throws exception

Property Axiom Semantics
------------------------

.. list-table::
   :header-rows: 1
   :widths: 15 20 65

   * - Done
     - Rule
     - Notes
   * - no
     - ``prp-ap``
     -
   * - yes
     - ``prp-dom``
     -
   * - yes
     - ``prp-rng``
     -
   * - yes
     - ``prp-fp``
     -
   * - yes
     - ``prp-ifp``
     -
   * - yes
     - ``prp-irp``
     - Throws exception
   * - yes
     - ``prp-symp``
     -
   * - yes
     - ``prp-asyp``
     - Throws exception
   * - yes
     - ``prp-trp``
     -
   * - yes
     - ``prp-spo1``
     -
   * - no
     - ``prp-spo2``
     -
   * - yes
     - ``prp-eqp1``
     -
   * - yes
     - ``prp-eqp2``
     -
   * - yes
     - ``prp-pdw``
     - Throws exception
   * - no
     - ``prp-adp``
     - Throws exception
   * - yes
     - ``prp-inv1``
     -
   * - yes
     - ``prp-inv2``
     -
   * - no
     - ``prp-key``
     -
   * - no
     - ``prp-npa1``
     - Throws exception
   * - no
     - ``prp-npa2``
     - Throws exception

Class Semantics
---------------

.. list-table::
   :header-rows: 1
   :widths: 15 20 65

   * - Done
     - Rule
     - Notes
   * - yes
     - ``cls-thing``
     -
   * - yes
     - ``cls-nothing1``
     -
   * - yes
     - ``cls-nothing2``
     - Throws exception
   * - yes
     - ``cls-int1``
     -
   * - yes
     - ``cls-int2``
     -
   * - yes
     - ``cls-uni``
     -
   * - yes
     - ``cls-com``
     - Throws exception
   * - yes
     - ``cls-svf1``
     -
   * - yes
     - ``cls-svf2``
     -
   * - yes
     - ``cls-avf``
     -
   * - yes
     - ``cls-hv1``
     -
   * - yes
     - ``cls-hv2``
     -
   * - no
     - ``cls-maxc1``
     - Throws exception
   * - no
     - ``cls-maxc2``
     -
   * - no
     - ``cls-maxqc1``
     - Throws exception
   * - no
     - ``cls-maxqc2``
     - Throws exception
   * - no
     - ``cls-maxqc3``
     -
   * - no
     - ``cls-maxqc4``
     -
   * - no
     - ``cls-oo``
     -

Class Axiom Semantics
---------------------

.. list-table::
   :header-rows: 1
   :widths: 15 20 65

   * - Done
     - Rule
     - Notes
   * - yes
     - ``cax-sco``
     -
   * - yes
     - ``cax-eqc1``
     -
   * - yes
     - ``cax-eqc2``
     -
   * - yes
     - ``cax-dw``
     - Throws exception
   * - no
     - ``cax-adc``
     - Throws exception

Schema Vocabulary Semantics
----------------------------

.. list-table::
   :header-rows: 1
   :widths: 15 20 65

   * - Done
     - Rule
     - Notes
   * - yes
     - ``scm-eqc1``
     - ``owl:equivalentClass`` → ``rdfs:subClassOf`` (one direction)
   * - yes
     - ``scm-eqc2``
     - ``owl:equivalentClass`` → ``rdfs:subClassOf`` (other direction)

Not yet implemented
-------------------

Datatype semantics are not currently implemented.
