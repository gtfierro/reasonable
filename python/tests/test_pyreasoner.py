import pytest
import rdflib


def test_import_and_metadata():
    import reasonable

    assert hasattr(reasonable, "PyReasoner")
    assert isinstance(reasonable.__version__, str)
    assert reasonable.__package__ == "reasonable"


def test_empty_reason_runs():
    import reasonable

    r = reasonable.PyReasoner()
    out = r.reason()
    assert isinstance(out, list)
    for t in out:
        assert isinstance(t, tuple)
        assert len(t) == 3


def test_from_graph_smoke_and_term_types():
    import reasonable
    from rdflib.namespace import XSD
    from rdflib.term import URIRef, BNode, Literal

    g = rdflib.Graph()
    g.add((URIRef("urn:s"), URIRef("urn:p"), URIRef("urn:o")))
    g.add((URIRef("urn:s2"), URIRef("urn:p2"), Literal("42", datatype=XSD.integer)))
    g.add((URIRef("urn:s3"), URIRef("urn:p3"), Literal("hello", lang="en")))

    r = reasonable.PyReasoner()
    r.from_graph(g)
    out = r.reason()
    assert isinstance(out, list)
    for t in out:
        assert isinstance(t, tuple) and len(t) == 3
        s, p, o = t
        assert isinstance(s, (URIRef, BNode))
        assert isinstance(p, URIRef)
        assert isinstance(o, (URIRef, BNode, Literal))


def test_load_file_missing_raises_ioerror():
    import reasonable

    r = reasonable.PyReasoner()
    with pytest.raises(OSError):
        r.load_file("this_file_does_not_exist_12345.ttl")


# -------------------- Ported tests from Rust (lib/src/tests.rs) --------------------

from rdflib import URIRef
from rdflib.namespace import RDF as RDFNS, RDFS as RDFSNS, OWL as OWLNS

# String constants mirroring Rust tests
RDFS_SUBCLASSOF = "http://www.w3.org/2000/01/rdf-schema#subClassOf"
RDFS_DOMAIN = "http://www.w3.org/2000/01/rdf-schema#domain"
RDFS_RANGE = "http://www.w3.org/2000/01/rdf-schema#range"
RDFS_LITERAL = "http://www.w3.org/2000/01/rdf-schema#Literal"
RDFS_RESOURCE = "http://www.w3.org/2000/01/rdf-schema#Resource"
RDFS_SUBPROP = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf"
RDF_TYPE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
RDF_FIRST = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first"
RDF_REST = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest"
RDF_NIL = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil"
OWL_SAMEAS = "http://www.w3.org/2002/07/owl#sameAs"
OWL_EQUIVALENTCLASS = "http://www.w3.org/2002/07/owl#equivalentClass"
OWL_HASVALUE = "http://www.w3.org/2002/07/owl#hasValue"
OWL_ALLVALUESFROM = "http://www.w3.org/2002/07/owl#allValuesFrom"
OWL_SOMEVALUESFROM = "http://www.w3.org/2002/07/owl#someValuesFrom"
OWL_ONPROPERTY = "http://www.w3.org/2002/07/owl#onProperty"
OWL_INVERSEOF = "http://www.w3.org/2002/07/owl#inverseOf"
OWL_SYMMETRICPROP = "http://www.w3.org/2002/07/owl#SymmetricProperty"
OWL_EQUIVPROP = "http://www.w3.org/2002/07/owl#equivalentProperty"
OWL_FUNCPROP = "http://www.w3.org/2002/07/owl#FunctionalProperty"
OWL_INVFUNCPROP = "http://www.w3.org/2002/07/owl#InverseFunctionalProperty"
OWL_TRANSPROP = "http://www.w3.org/2002/07/owl#TransitiveProperty"
OWL_INTERSECTION = "http://www.w3.org/2002/07/owl#intersectionOf"
OWL_UNION = "http://www.w3.org/2002/07/owl#unionOf"
OWL_CLASS = "http://www.w3.org/2002/07/owl#Class"
OWL_THING = "http://www.w3.org/2002/07/owl#Thing"
OWL_NOTHING = "http://www.w3.org/2002/07/owl#Nothing"
OWL_COMPLEMENT = "http://www.w3.org/2002/07/owl#complementOf"
OWL_RESTRICTION = "http://www.w3.org/2002/07/owl#Restriction"
OWL_ASYMMETRICPROP = "http://www.w3.org/2002/07/owl#AsymmetricProperty"


def _reason_over_str_triples(triples):
    import reasonable
    g = rdflib.Graph()
    for s, p, o in triples:
        g.add((URIRef(s), URIRef(p), URIRef(o)))
    r = reasonable.PyReasoner()
    r.from_graph(g)
    return set(r.reason())


def test_eq_sym():
    out = _reason_over_str_triples([
        ("urn:x", OWL_SAMEAS, "urn:y"),
    ])
    assert (URIRef("urn:y"), URIRef(OWL_SAMEAS), URIRef("urn:x")) in out


def test_eq_trans():
    out = _reason_over_str_triples([
        ("urn:x", OWL_SAMEAS, "urn:y"),
        ("urn:y", OWL_SAMEAS, "urn:z"),
    ])
    assert (URIRef("urn:x"), URIRef(OWL_SAMEAS), URIRef("urn:z")) in out


def test_eq_rep_s():
    out = _reason_over_str_triples([
        ("urn:s1", OWL_SAMEAS, "urn:s2"),
        ("urn:s1", "urn:p", "urn:o"),
    ])
    assert (URIRef("urn:s2"), URIRef("urn:p"), URIRef("urn:o")) in out


def test_eq_rep_p():
    out = _reason_over_str_triples([
        ("urn:p1", OWL_SAMEAS, "urn:p2"),
        ("urn:s", "urn:p1", "urn:o"),
    ])
    assert (URIRef("urn:s"), URIRef("urn:p2"), URIRef("urn:o")) in out


def test_eq_rep_o():
    out = _reason_over_str_triples([
        ("urn:o1", OWL_SAMEAS, "urn:o2"),
        ("urn:s", "urn:p", "urn:o1"),
    ])
    assert (URIRef("urn:s"), URIRef("urn:p"), URIRef("urn:o2")) in out


def test_cax_sco():
    out = _reason_over_str_triples([
        ("urn:Class2", RDFS_SUBCLASSOF, "urn:Class1"),
        ("urn:a", RDF_TYPE, "urn:Class2"),
    ])
    assert (URIRef("urn:a"), URIRef(RDF_TYPE), URIRef("urn:Class1")) in out


def test_cax_eqc1():
    out = _reason_over_str_triples([
        ("urn:Class1", OWL_EQUIVALENTCLASS, "urn:Class2"),
        ("urn:a", RDF_TYPE, "urn:Class1"),
    ])
    assert (URIRef("urn:a"), URIRef(RDF_TYPE), URIRef("urn:Class2")) in out


def test_cax_eqc2():
    out = _reason_over_str_triples([
        ("urn:Class1", OWL_EQUIVALENTCLASS, "urn:Class2"),
        ("urn:a", RDF_TYPE, "urn:Class2"),
    ])
    assert (URIRef("urn:a"), URIRef(RDF_TYPE), URIRef("urn:Class1")) in out


def test_cax_eqc_chain_1():
    out = _reason_over_str_triples([
        ("urn:Class1", OWL_EQUIVALENTCLASS, "urn:Class2"),
        ("urn:Class2", OWL_EQUIVALENTCLASS, "urn:Class3"),
        ("urn:Class3", OWL_EQUIVALENTCLASS, "urn:Class4"),
        ("urn:Class4", OWL_EQUIVALENTCLASS, "urn:Class5"),
        ("urn:Class5", OWL_EQUIVALENTCLASS, "urn:Class6"),
        ("urn:a", RDF_TYPE, "urn:Class1"),
    ])
    for cls in ["urn:Class2", "urn:Class3", "urn:Class4", "urn:Class5", "urn:Class6"]:
        assert (URIRef("urn:a"), URIRef(RDF_TYPE), URIRef(cls)) in out


def test_cax_eqc_chain_2():
    out = _reason_over_str_triples([
        ("urn:Class1", OWL_EQUIVALENTCLASS, "urn:Class2"),
        ("urn:Class2", OWL_EQUIVALENTCLASS, "urn:Class3"),
        ("urn:Class3", OWL_EQUIVALENTCLASS, "urn:Class4"),
        ("urn:Class4", OWL_EQUIVALENTCLASS, "urn:Class5"),
        ("urn:Class5", OWL_EQUIVALENTCLASS, "urn:Class6"),
        ("urn:a", RDF_TYPE, "urn:Class6"),
    ])
    for cls in ["urn:Class1", "urn:Class2", "urn:Class3", "urn:Class4", "urn:Class5"]:
        assert (URIRef("urn:a"), URIRef(RDF_TYPE), URIRef(cls)) in out


def test_prp_fp():
    out = _reason_over_str_triples([
        ("urn:PRED", RDF_TYPE, OWL_FUNCPROP),
        ("urn:SUB", "urn:PRED", "urn:OBJECT_1"),
        ("urn:SUB", "urn:PRED", "urn:OBJECT_2"),
    ])
    assert (URIRef("urn:OBJECT_1"), URIRef(OWL_SAMEAS), URIRef("urn:OBJECT_2")) in out


def test_prp_fp_2():
    out = _reason_over_str_triples([
        ("urn:PRED", RDF_TYPE, OWL_FUNCPROP),
        ("urn:SUB", "urn:PRED", "urn:OBJECT_1"),
        ("urn:SUB1", "urn:PRED", "urn:OBJECT_2"),
    ])
    assert (URIRef("urn:OBJECT_1"), URIRef(OWL_SAMEAS), URIRef("urn:OBJECT_2")) not in out


def test_prp_ifp():
    out = _reason_over_str_triples([
        ("urn:PRED", RDF_TYPE, OWL_INVFUNCPROP),
        ("urn:SUB_1", "urn:PRED", "urn:OBJECT"),
        ("urn:SUB_2", "urn:PRED", "urn:OBJECT"),
    ])
    assert (URIRef("urn:SUB_1"), URIRef(OWL_SAMEAS), URIRef("urn:SUB_2")) in out


def test_spo1():
    out = _reason_over_str_triples([
        ("urn:p1", RDFS_SUBPROP, "urn:p2"),
        ("urn:x", "urn:p1", "urn:y"),
    ])
    assert (URIRef("urn:x"), URIRef("urn:p2"), URIRef("urn:y")) in out


def test_prp_inv1():
    out = _reason_over_str_triples([
        ("urn:p1", OWL_INVERSEOF, "urn:p2"),
        ("urn:x", "urn:p1", "urn:y"),
    ])
    assert (URIRef("urn:y"), URIRef("urn:p2"), URIRef("urn:x")) in out


def test_prp_inv2():
    out = _reason_over_str_triples([
        ("urn:p1", OWL_INVERSEOF, "urn:p2"),
        ("urn:y", "urn:p2", "urn:x"),
    ])
    assert (URIRef("urn:x"), URIRef("urn:p1"), URIRef("urn:y")) in out


def test_prp_symp():
    out = _reason_over_str_triples([
        ("urn:p", RDF_TYPE, OWL_SYMMETRICPROP),
        ("urn:x", "urn:p", "urn:y"),
    ])
    assert (URIRef("urn:y"), URIRef("urn:p"), URIRef("urn:x")) in out


def test_prp_trp():
    out = _reason_over_str_triples([
        ("urn:p", RDF_TYPE, OWL_TRANSPROP),
        ("urn:x", "urn:p", "urn:y"),
        ("urn:y", "urn:p", "urn:z"),
    ])
    assert (URIRef("urn:x"), URIRef("urn:p"), URIRef("urn:z")) in out


def test_prp_eqp1():
    out = _reason_over_str_triples([
        ("urn:p1", OWL_EQUIVPROP, "urn:p2"),
        ("urn:x", "urn:p1", "urn:y"),
    ])
    assert (URIRef("urn:x"), URIRef("urn:p2"), URIRef("urn:y")) in out


def test_cls_thing_nothing():
    out = _reason_over_str_triples([])
    assert (OWLNS.Thing, RDFNS.type, OWLNS.Class) in out
    assert (OWLNS.Nothing, RDFNS.type, OWLNS.Class) in out


def test_cls_hv1():
    out = _reason_over_str_triples([
        ("urn:x", OWL_HASVALUE, "urn:y"),
        ("urn:x", OWL_ONPROPERTY, "urn:p"),
        ("urn:u", RDF_TYPE, "urn:x"),
    ])
    assert (URIRef("urn:u"), URIRef("urn:p"), URIRef("urn:y")) in out


def test_cls_hv2():
    out = _reason_over_str_triples([
        ("urn:x", OWL_HASVALUE, "urn:y"),
        ("urn:x", OWL_ONPROPERTY, "urn:p"),
        ("urn:u", "urn:p", "urn:y"),
    ])
    assert (URIRef("urn:u"), URIRef(RDF_TYPE), URIRef("urn:x")) in out


def test_cls_avf():
    out = _reason_over_str_triples([
        ("urn:x", OWL_ALLVALUESFROM, "urn:y"),
        ("urn:x", OWL_ONPROPERTY, "urn:p"),
        ("urn:u", RDF_TYPE, "urn:x"),
        ("urn:u", "urn:p", "urn:v"),
    ])
    assert (URIRef("urn:v"), URIRef(RDF_TYPE), URIRef("urn:y")) in out


def test_cls_svf1():
    out = _reason_over_str_triples([
        ("urn:x", OWL_SOMEVALUESFROM, "urn:y"),
        ("urn:x", OWL_ONPROPERTY, "urn:p"),
        ("urn:u", "urn:p", "urn:v"),
        ("urn:v", RDF_TYPE, "urn:y"),
    ])
    assert (URIRef("urn:u"), URIRef(RDF_TYPE), URIRef("urn:x")) in out


def test_cls_svf2():
    out = _reason_over_str_triples([
        ("urn:x", OWL_SOMEVALUESFROM, OWL_THING),
        ("urn:x", OWL_ONPROPERTY, "urn:p"),
        ("urn:u", "urn:p", "urn:v"),
    ])
    assert (URIRef("urn:u"), URIRef(RDF_TYPE), URIRef("urn:x")) in out


def test_cls_int1():
    out = _reason_over_str_triples([
        ("urn:c", OWL_INTERSECTION, "urn:x"),
        ("urn:x", RDF_FIRST, "urn:c1"),
        ("urn:x", RDF_REST, "urn:z2"),
        ("urn:z2", RDF_FIRST, "urn:c2"),
        ("urn:z2", RDF_REST, "urn:z3"),
        ("urn:z3", RDF_FIRST, "urn:c3"),
        ("urn:z3", RDF_REST, RDF_NIL),
        ("urn:y", RDF_TYPE, "urn:c1"),
        ("urn:y", RDF_TYPE, "urn:c2"),
        ("urn:y", RDF_TYPE, "urn:c3"),
    ])
    assert (URIRef("urn:y"), URIRef(RDF_TYPE), URIRef("urn:c")) in out


def test_cls_int2():
    out = _reason_over_str_triples([
        ("urn:c", OWL_INTERSECTION, "urn:x"),
        ("urn:x", RDF_FIRST, "urn:c1"),
        ("urn:x", RDF_REST, "urn:z2"),
        ("urn:z2", RDF_FIRST, "urn:c2"),
        ("urn:z2", RDF_REST, "urn:z3"),
        ("urn:z3", RDF_FIRST, "urn:c3"),
        ("urn:z3", RDF_REST, RDF_NIL),
        ("urn:y", RDF_TYPE, "urn:c"),
    ])
    for cls in ["urn:c1", "urn:c2", "urn:c3"]:
        assert (URIRef("urn:y"), URIRef(RDF_TYPE), URIRef(cls)) in out


def test_cls_int2_withequivalent():
    out = _reason_over_str_triples([
        ("urn:c", OWL_INTERSECTION, "urn:x"),
        ("urn:x", RDF_FIRST, "urn:c1"),
        ("urn:x", RDF_REST, "urn:z2"),
        ("urn:z2", RDF_FIRST, "urn:c2"),
        ("urn:z2", RDF_REST, "urn:z3"),
        ("urn:z3", RDF_FIRST, "urn:c3"),
        ("urn:z3", RDF_REST, RDF_NIL),
        ("urn:y", RDF_TYPE, "urn:c"),
        ("urn:c", OWL_EQUIVALENTCLASS, "urn:C"),
        ("urn:C", OWL_INTERSECTION, "urn:X"),
        ("urn:X", RDF_FIRST, "urn:C1"),
        ("urn:X", RDF_REST, "urn:Z2"),
        ("urn:Z2", RDF_FIRST, "urn:C2"),
        ("urn:Z2", RDF_REST, "urn:Z3"),
        ("urn:Z3", RDF_FIRST, "urn:C3"),
        ("urn:Z3", RDF_REST, RDF_NIL),
    ])
    for cls in ["urn:c1", "urn:c2", "urn:c3", "urn:C1", "urn:C2", "urn:C3"]:
        assert (URIRef("urn:y"), URIRef(RDF_TYPE), URIRef(cls)) in out


def test_cls_int1_withhasvalue():
    out = _reason_over_str_triples([
        ("urn:intersection_class", OWL_INTERSECTION, "urn:x"),
        ("urn:x", RDF_FIRST, "urn:c1"),
        ("urn:x", RDF_REST, "urn:z2"),
        ("urn:z2", RDF_FIRST, "urn:c2"),
        ("urn:z2", RDF_REST, RDF_NIL),
        ("urn:c1", OWL_HASVALUE, "urn:c1p_value"),
        ("urn:c1", OWL_ONPROPERTY, "urn:c1p"),
        ("urn:c2", OWL_HASVALUE, "urn:c2p_value"),
        ("urn:c2", OWL_ONPROPERTY, "urn:c2p"),
        ("urn:inst", "urn:c1p", "urn:c1p_value"),
        ("urn:inst", "urn:c2p", "urn:c2p_value"),
    ])
    assert (URIRef("urn:inst"), URIRef(RDF_TYPE), URIRef("urn:c1")) in out
    assert (URIRef("urn:inst"), URIRef(RDF_TYPE), URIRef("urn:c2")) in out
    assert (URIRef("urn:inst"), URIRef(RDF_TYPE), URIRef("urn:intersection_class")) in out


def test_complementof():
    out = _reason_over_str_triples([
        ("urn:c", OWL_EQUIVALENTCLASS, "urn:c2"),
        ("urn:c2", OWL_COMPLEMENT, "urn:x"),
        ("urn:x", OWL_HASVALUE, "urn:v"),
        ("urn:x", OWL_ONPROPERTY, "urn:p"),
        ("urn:inst1", "urn:p", "urn:v"),
        ("urn:inst2", "urn:p", "urn:v2"),
        ("urn:x", RDF_TYPE, OWL_CLASS),
        ("urn:c", RDF_TYPE, OWL_CLASS),
        ("urn:c2", RDF_TYPE, OWL_CLASS),
        ("urn:inst2", RDF_TYPE, OWL_THING),
    ])
    assert (URIRef("urn:inst1"), URIRef(RDF_TYPE), OWLNS.Thing) in out
    assert (URIRef("urn:inst1"), URIRef(RDF_TYPE), URIRef("urn:x")) in out
    assert (URIRef("urn:inst1"), URIRef(RDF_TYPE), URIRef("urn:c")) not in out
    assert (URIRef("urn:inst1"), URIRef(RDF_TYPE), URIRef("urn:c2")) not in out
    assert (URIRef("urn:inst2"), URIRef(RDF_TYPE), URIRef("urn:c2")) in out


def test_error_asymmetric():
    # Just ensure reasoning runs; error collection is internal
    _ = _reason_over_str_triples([
        ("urn:p", RDF_TYPE, OWL_ASYMMETRICPROP),
        ("urn:x", "urn:p", "urn:y"),
        ("urn:y", "urn:p", "urn:x"),
    ])
