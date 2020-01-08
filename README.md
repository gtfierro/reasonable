## OWL 2 Rules

Using rule definitions from [here](https://www.w3.org/TR/owl2-profiles/#Reasoning_in_OWL_2_RL_and_RDF_Graphs_using_Rules)

### Equality Semantics

|Completed| Rule name | Notes |
|---------|----------|-------|
| **yes**| `eq-ref` |       |
| **yes**| `eq-sym` |       |
| **yes**| `eq-trans` |       |
| **yes**| `eq-rep-s` |       |
| **yes**| `eq-rep-p` |       |
| **yes**| `eq-rep-o` |       |
| no     | `eq-diff1` |       |
| no     | `eq-diff2` |       |
| no     | `eq-diff3` |       |

### Property Axiom Semantics

|Completed| Rule name | Notes |
|---------|----------|-------|
| no        | `prp-ap` |       |
| no        | `prp-dom` |       |
| no        | `prp-rng` |       |
| **yes**   | `prp-fp` |       |
| **yes**   | `prp-ifp` |       |
| no        | `prp-irp` |       |
| **yes**   | `prp-symp` |       |
| no        | `prp-asyp` |       |
| no        | `prp-trp` |       |
| **yes**   | `prp-spo1` |       |
| *almost*  | `prp-spo2` |       |
| **yes**   | `prp-eqp1` |       |
| **yes**   | `prp-eqp2` |       |
| no        | `prp-pdw` |       |
| no        | `prp-adp` |       |
| **yes**   | `prp-inv1` |       |
| **yes**   | `prp-inv2` |       |
| no        | `prp-key` |       |
| no        | `prp-npa1` |       |
| no        | `prp-npa2` |       |

### Class Semantics

|Completed| Rule name | Notes |
|---------|----------|-------|
| no     | `cls-thing` |       |
| no     | `cls-nothing1` |       |
| no     | `cls-nothing2` |       |
| **yes**| `cls-int1` |       |
| **yes**| `cls-int2` |       |
| no     | `cls-uni` |       |
| no     | `cls-com` |       |
| **yes**| `cls-svf1` |       |
| no     | `cls-svf2` |       |
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
