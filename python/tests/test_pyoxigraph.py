"""Tests for the pyoxigraph-compatible API: add_quads, update_quads, reason_quads."""
import pytest

pyoxigraph = pytest.importorskip("pyoxigraph")
from pyoxigraph import (
    BlankNode,
    DefaultGraph,
    Literal,
    NamedNode,
    Quad,
    Store,
    Triple,
)


RDFS_SUBCLASSOF = "http://www.w3.org/2000/01/rdf-schema#subClassOf"
RDF_TYPE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
OWL_SAMEAS = "http://www.w3.org/2002/07/owl#sameAs"
XSD_INTEGER = "http://www.w3.org/2001/XMLSchema#integer"


def _quad_tuples(quads):
    """Reduce pyoxigraph quads to (s, p, o) string tuples for set comparison."""
    out = set()
    for q in quads:
        out.add((str(q.subject.value), str(q.predicate.value), _term_key(q.object)))
    return out


def _term_key(t):
    # Literals stringify to their N-Triples form; just use value for comparison
    if isinstance(t, Literal):
        return ("lit", t.value, t.datatype.value, t.language)
    return ("iri", t.value)


def test_add_quads_from_triples_then_reason_quads():
    import reasonable

    r = reasonable.PyReasoner()
    r.add_quads(
        [
            Triple(NamedNode("urn:A"), NamedNode(RDFS_SUBCLASSOF), NamedNode("urn:B")),
            Triple(NamedNode("urn:x"), NamedNode(RDF_TYPE), NamedNode("urn:A")),
        ]
    )
    out = r.reason_quads()
    assert all(isinstance(q, Quad) for q in out)
    # Every quad sits in the default graph by default
    assert all(isinstance(q.graph_name, DefaultGraph) for q in out)
    keys = _quad_tuples(out)
    assert (("iri", "urn:x"), "predicate-placeholder", ("iri", "urn:B")) not in keys
    # The real check: x rdf:type B was inferred
    assert (
        ("iri", "urn:x"),
        ("iri", RDF_TYPE),
    ) == (("iri", "urn:x"), ("iri", RDF_TYPE))  # sanity
    found = any(
        q.subject == NamedNode("urn:x")
        and q.predicate == NamedNode(RDF_TYPE)
        and q.object == NamedNode("urn:B")
        for q in out
    )
    assert found, "expected inferred (urn:x rdf:type urn:B)"


def test_add_quads_from_quads_ignores_graph_name():
    import reasonable

    g = NamedNode("urn:source-graph")
    quads = [
        Quad(NamedNode("urn:A"), NamedNode(RDFS_SUBCLASSOF), NamedNode("urn:B"), g),
        Quad(NamedNode("urn:x"), NamedNode(RDF_TYPE), NamedNode("urn:A"), g),
    ]
    r = reasonable.PyReasoner()
    r.add_quads(quads)
    out = r.reason_quads(graph_name=NamedNode("urn:inferred"))
    found = any(
        q.subject == NamedNode("urn:x")
        and q.predicate == NamedNode(RDF_TYPE)
        and q.object == NamedNode("urn:B")
        and q.graph_name == NamedNode("urn:inferred")
        for q in out
    )
    assert found


def test_update_quads_additions_only_is_incremental():
    import reasonable

    r = reasonable.PyReasoner()
    base = [
        Triple(NamedNode("urn:A"), NamedNode(RDFS_SUBCLASSOF), NamedNode("urn:B")),
    ]
    needs_full = r.update_quads(base)
    assert needs_full is False
    r.reason_quads()

    extended = base + [
        Triple(NamedNode("urn:x"), NamedNode(RDF_TYPE), NamedNode("urn:A")),
    ]
    needs_full = r.update_quads(extended)
    assert needs_full is False, "pure addition should be incremental"


def test_update_quads_removal_triggers_full():
    import reasonable

    r = reasonable.PyReasoner()
    base = [
        Triple(NamedNode("urn:A"), NamedNode(RDFS_SUBCLASSOF), NamedNode("urn:B")),
        Triple(NamedNode("urn:x"), NamedNode(RDF_TYPE), NamedNode("urn:A")),
    ]
    r.update_quads(base)
    out = r.reason_quads()
    assert any(
        q.subject == NamedNode("urn:x") and q.object == NamedNode("urn:B")
        for q in out
    )

    shrunk = base[:1]
    needs_full = r.update_quads(shrunk)
    assert needs_full is True
    out = r.reason_quads()
    assert not any(
        q.subject == NamedNode("urn:x") and q.object == NamedNode("urn:B")
        for q in out
    )


def test_literals_roundtrip_typed_and_lang():
    import reasonable

    typed = Literal("42", datatype=NamedNode(XSD_INTEGER))
    lang = Literal("hello", language="en")
    r = reasonable.PyReasoner()
    r.add_quads(
        [
            Triple(NamedNode("urn:s"), NamedNode("urn:p_int"), typed),
            Triple(NamedNode("urn:s"), NamedNode("urn:p_str"), lang),
        ]
    )
    out = r.reason_quads()
    objs_by_pred = {q.predicate.value: q.object for q in out if q.subject == NamedNode("urn:s")}
    got_typed = objs_by_pred["urn:p_int"]
    got_lang = objs_by_pred["urn:p_str"]
    assert isinstance(got_typed, Literal)
    assert got_typed.value == "42"
    assert got_typed.datatype == NamedNode(XSD_INTEGER)
    assert isinstance(got_lang, Literal)
    assert got_lang.value == "hello"
    assert got_lang.language == "en"


def test_blank_nodes_roundtrip():
    import reasonable

    b = BlankNode("b1")
    r = reasonable.PyReasoner()
    r.add_quads([Triple(b, NamedNode("urn:p"), NamedNode("urn:o"))])
    out = r.reason_quads()
    assert any(isinstance(q.subject, BlankNode) for q in out)


def test_store_workflow_matches_issue_example():
    """Mirror the workflow from issue #54: ingest a pyoxigraph Store and write
    inferred quads back into a named graph."""
    import reasonable

    store = Store()
    store.add(
        Quad(
            NamedNode("urn:A"),
            NamedNode(RDFS_SUBCLASSOF),
            NamedNode("urn:B"),
            DefaultGraph(),
        )
    )
    store.add(
        Quad(
            NamedNode("urn:x"),
            NamedNode(RDF_TYPE),
            NamedNode("urn:A"),
            DefaultGraph(),
        )
    )

    reasoned_graph = NamedNode("urn:reasoned")
    r = reasonable.PyReasoner()
    r.update_quads(store.quads_for_pattern(None, None, None, DefaultGraph()))
    inferred = r.reason_quads(graph_name=reasoned_graph)

    store.bulk_extend(inferred)
    materialized = list(
        store.quads_for_pattern(NamedNode("urn:x"), NamedNode(RDF_TYPE), NamedNode("urn:B"), reasoned_graph)
    )
    assert len(materialized) == 1


def test_invalid_iterable_element_raises_typeerror():
    import reasonable

    r = reasonable.PyReasoner()
    with pytest.raises(TypeError):
        r.add_quads([("urn:s", "urn:p", "urn:o")])
