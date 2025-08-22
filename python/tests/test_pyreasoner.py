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
