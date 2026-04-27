OWL 2 RL Rules
==============

Reasonable implements a subset of the
`OWL 2 RL reasoning rules <https://www.w3.org/TR/owl2-profiles/#Reasoning_in_OWL_2_RL_and_RDF_Graphs_using_Rules>`_
defined by the W3C.  The tables below show the current implementation
status.

.. list-table::
   :widths: auto

   * - :supported:`supported`
     - Rule is fully implemented and produces inferred triples.
   * - :partial:`partial`
     - Rule is implemented but produces a diagnostic instead of new triples
       (violation detection only).
   * - :unsupported:`not supported`
     - Rule is not yet implemented.

RDFS Semantics
--------------

.. list-table::
   :header-rows: 1
   :widths: 18 22 60

   * - Status
     - Rule
     - Notes
   * - :supported:`supported`
     - ``rdfs11``
     - ``rdfs:subClassOf`` transitivity

Equality Semantics
------------------

.. list-table::
   :header-rows: 1
   :widths: 18 22 60

   * - Status
     - Rule
     - Notes
   * - :unsupported:`not supported`
     - ``eq-ref``
     - Very inefficient — causes runaway flux; not implemented
   * - :supported:`supported`
     - ``eq-sym``
     -
   * - :supported:`supported`
     - ``eq-trans``
     -
   * - :supported:`supported`
     - ``eq-rep-s``
     -
   * - :supported:`supported`
     - ``eq-rep-p``
     -
   * - :supported:`supported`
     - ``eq-rep-o``
     -
   * - :unsupported:`not supported`
     - ``eq-diff1``
     -
   * - :unsupported:`not supported`
     - ``eq-diff2``
     -
   * - :unsupported:`not supported`
     - ``eq-diff3``
     -

Property Axiom Semantics
------------------------

.. list-table::
   :header-rows: 1
   :widths: 18 22 60

   * - Status
     - Rule
     - Notes
   * - :unsupported:`not supported`
     - ``prp-ap``
     -
   * - :supported:`supported`
     - ``prp-dom``
     -
   * - :supported:`supported`
     - ``prp-rng``
     -
   * - :supported:`supported`
     - ``prp-fp``
     -
   * - :supported:`supported`
     - ``prp-ifp``
     -
   * - :partial:`partial`
     - ``prp-irp``
     - Violation detection only — emits a diagnostic
   * - :supported:`supported`
     - ``prp-symp``
     -
   * - :partial:`partial`
     - ``prp-asyp``
     - Violation detection only — emits a diagnostic
   * - :supported:`supported`
     - ``prp-trp``
     -
   * - :supported:`supported`
     - ``prp-spo1``
     -
   * - :unsupported:`not supported`
     - ``prp-spo2``
     -
   * - :supported:`supported`
     - ``prp-eqp1``
     -
   * - :supported:`supported`
     - ``prp-eqp2``
     -
   * - :partial:`partial`
     - ``prp-pdw``
     - Violation detection only — emits a diagnostic
   * - :unsupported:`not supported`
     - ``prp-adp``
     -
   * - :supported:`supported`
     - ``prp-inv1``
     -
   * - :supported:`supported`
     - ``prp-inv2``
     -
   * - :unsupported:`not supported`
     - ``prp-key``
     -
   * - :unsupported:`not supported`
     - ``prp-npa1``
     -
   * - :unsupported:`not supported`
     - ``prp-npa2``
     -

Class Semantics
---------------

.. list-table::
   :header-rows: 1
   :widths: 18 22 60

   * - Status
     - Rule
     - Notes
   * - :supported:`supported`
     - ``cls-thing``
     -
   * - :supported:`supported`
     - ``cls-nothing1``
     -
   * - :partial:`partial`
     - ``cls-nothing2``
     - Violation detection only — emits a diagnostic
   * - :supported:`supported`
     - ``cls-int1``
     -
   * - :supported:`supported`
     - ``cls-int2``
     -
   * - :supported:`supported`
     - ``cls-uni``
     -
   * - :partial:`partial`
     - ``cls-com``
     - Violation detection only — emits a diagnostic
   * - :supported:`supported`
     - ``cls-svf1``
     -
   * - :supported:`supported`
     - ``cls-svf2``
     -
   * - :supported:`supported`
     - ``cls-avf``
     -
   * - :supported:`supported`
     - ``cls-hv1``
     -
   * - :supported:`supported`
     - ``cls-hv2``
     -
   * - :unsupported:`not supported`
     - ``cls-maxc1``
     -
   * - :unsupported:`not supported`
     - ``cls-maxc2``
     -
   * - :unsupported:`not supported`
     - ``cls-maxqc1``
     -
   * - :unsupported:`not supported`
     - ``cls-maxqc2``
     -
   * - :unsupported:`not supported`
     - ``cls-maxqc3``
     -
   * - :unsupported:`not supported`
     - ``cls-maxqc4``
     -
   * - :unsupported:`not supported`
     - ``cls-oo``
     -

Class Axiom Semantics
---------------------

.. list-table::
   :header-rows: 1
   :widths: 18 22 60

   * - Status
     - Rule
     - Notes
   * - :supported:`supported`
     - ``cax-sco``
     -
   * - :supported:`supported`
     - ``cax-eqc1``
     -
   * - :supported:`supported`
     - ``cax-eqc2``
     -
   * - :partial:`partial`
     - ``cax-dw``
     - Violation detection only — emits a diagnostic
   * - :unsupported:`not supported`
     - ``cax-adc``
     -

Schema Vocabulary Semantics
----------------------------

.. list-table::
   :header-rows: 1
   :widths: 18 22 60

   * - Status
     - Rule
     - Notes
   * - :supported:`supported`
     - ``scm-eqc1``
     - ``owl:equivalentClass`` → ``rdfs:subClassOf`` (one direction)
   * - :supported:`supported`
     - ``scm-eqc2``
     - ``owl:equivalentClass`` → ``rdfs:subClassOf`` (other direction)

Not yet implemented
-------------------

Datatype semantics are not currently implemented.
