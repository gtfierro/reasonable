## OWL 2 Rules

Using rule definitions from [here](https://www.w3.org/TR/owl2-profiles/#Reasoning_in_OWL_2_RL_and_RDF_Graphs_using_Rules)

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
| no     | `cls-maxc1` |       |
| no     | `cls-maxc2` |       |
| no     | `cls-maxqc1` |       |
| no     | `cls-maxqc2` |       |
| no     | `cls-maxqc3` |       |
| no     | `cls-maxqc4` |       |
| no     | `cls-oo` |       |

### Class Axiom Semantics

|Completed| Rule name | Notes |
|---------|----------|-------|
| **yes**| `cax-sco` |       |
| **yes**| `cax-eqc1` |       |
| **yes**| `cax-eqc2` |       |
| **yes**| `cax-dw` |       |
| no     | `cax-adc` |       |

### Other

- no datatype semantics for now
