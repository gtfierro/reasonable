# Reasonable

An OWL 2 RL reasoner with reasonable performance

## Implementation Notes


### `owl:complementOf` complications

A modeling complication introduced by the formalization of the mapping between tags (`owl:hasValue <tag>; owl:onProperty brick:hasTag`)
is logical conflict between the disjointness of the Brick `Point` classes (`Alarm`, `Setpoint`, `Sensor` and so on) and what is implied by
the tags.

Consider the two classes `Air_Flow_Setpoint` (a subclass of `Setpoint`) and `Max_Air_Flow_Setpoint_Limit` (a subclass of `Parameter`).
tags for `Air_Flow_Setpoint` are a subset of the tags for `Max_Air_Flow_Setpoint_Limit`, so `Max_Air_Flow_Setpoint_Limit` is inferred
as a `Setpoint` in addition to a `Parameter`.

To get around this, we introduce an additional constraint that certain classes can *not* have certain tags. This is accomplished
by mandating that the class definitions intersect with the set of entities that are complementary to the set that has the forbidden tag.
For example:

```ttl
brick:Setpoint a owl:Class ;
    rdfs:label "Setpoint" ;
    rdfs:subClassOf brick:Point ;
    owl:equivalentClass [ owl:intersectionOf (
                            [ a owl:Class ; owl:complementOf [
                                a owl:Restriction ;
                                owl:hasValue tag:Parameter ;
                                owl:onProperty brick:hasTag ] ]
                            [ a owl:Restriction ;
                                owl:hasValue tag:Setpoint ;
                                owl:onProperty brick:hasTag ]
                        ) ] .
```

The issue now is how to efficiently and correctly implement `owl:complementOf`. The entailment semantics given in the W3C document
only flag logical conflicts.

## OWL 2 Rules

Using rule definitions from [here](https://www.w3.org/TR/owl2-profiles/#Reasoning_in_OWL_2_RL_and_RDF_Graphs_using_Rules).

**TODO**: implement RDF/RDFS entailment semantics as described [here](https://www.w3.org/TR/rdf11-mt/)

**Note**: haven't implemented rules that produce exceptions; waiting to determine the best way of handling these errors.

### Equality Semantics

|Completed| Rule name | Notes |
|---------|----------|-------|
| no     | `eq-ref` | implementation is very inefficient; causes lots of flux       |
| **yes**| `eq-sym` |       |
| **yes**| `eq-trans` |       |
| **yes**| `eq-rep-s` |       |
| **yes**| `eq-rep-p` |       |
| **yes**| `eq-rep-o` |       |
| no     | `eq-diff1` | `eq-diff{1,2,3}` all involve throwing exceptions. Not clear how we want to handle these yet      |
| no     | `eq-diff2` | throws exception |
| no     | `eq-diff3` | throws exception |

### Property Axiom Semantics

|Completed| Rule name | Notes |
|---------|----------|-------|
| no        | `prp-ap` |       |
| **yes**   | `prp-dom` |       |
| **yes**   | `prp-rng` |       |
| **yes**   | `prp-fp` |       |
| **yes**   | `prp-ifp` |       |
| no        | `prp-irp` | throws exception |
| **yes**   | `prp-symp` |       |
| no        | `prp-asyp` | throws exception |
| **yes**   | `prp-trp` |       |
| **yes**   | `prp-spo1` |       |
| no        | `prp-spo2` |       |
| **yes**   | `prp-eqp1` |       |
| **yes**   | `prp-eqp2` |       |
| no        | `prp-pdw` | throws exception |
| no        | `prp-adp` | throws exception |
| **yes**   | `prp-inv1` |       |
| **yes**   | `prp-inv2` |       |
| no        | `prp-key` |       |
| no        | `prp-npa1` | throws exception |
| no        | `prp-npa2` | throws exception |

### Class Semantics

|Completed| Rule name | Notes |
|---------|----------|-------|
| **yes**| `cls-thing` |       |
| **yes**| `cls-nothing1` |       |
| no     | `cls-nothing2` | throws exception       |
| **yes**| `cls-int1` |       |
| **yes**| `cls-int2` |       |
| **yes**| `cls-uni` |       |
| no     | `cls-com` | throws exception    |
| **yes**| `cls-svf1` |       |
| **yes**| `cls-svf2` |       |
| **yes**| `cls-avf` |       |
| **yes**| `cls-hv1` |       |
| **yes**| `cls-hv2` |       |
| no     | `cls-maxc1` | throws exception       |
| no     | `cls-maxc2` |       |
| no     | `cls-maxqc1` | throws exception       |
| no     | `cls-maxqc2` | throws exception      |
| no     | `cls-maxqc3` |       |
| no     | `cls-maxqc4` |       |
| no     | `cls-oo` |       |

### Class Axiom Semantics

|Completed| Rule name | Notes |
|---------|----------|-------|
| **yes**| `cax-sco` |       |
| **yes**| `cax-eqc1` |       |
| **yes**| `cax-eqc2` |       |
| **yes**| `cax-dw` | throws exception      |
| no     | `cax-adc` |  throws exception     |

### Other

- no datatype semantics for now
