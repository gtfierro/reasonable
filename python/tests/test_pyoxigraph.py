import sys

import pytest
from pyoxigraph import BaseDirection, BlankNode, DefaultGraph, Literal, NamedNode, Quad, Store, Triple

from reasonable import PyReasoner


RDF_TYPE = NamedNode("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
SUBCLASS_OF = NamedNode("http://www.w3.org/2000/01/rdf-schema#subClassOf")
PERSON = NamedNode("urn:Person")
AGENT = NamedNode("urn:Agent")
ALICE = NamedNode("urn:alice")
BOB = NamedNode("urn:bob")


def test_refresh_a_store_graph_from_a_selected_source():
    store = Store()
    target = NamedNode("urn:reasoned")
    store.extend([
        Quad(PERSON, SUBCLASS_OF, AGENT),
        Quad(ALICE, RDF_TYPE, PERSON),
        Quad(BOB, RDF_TYPE, PERSON, NamedNode("urn:unrelated")),
    ])
    reasoner = PyReasoner()

    def refresh():
        removed = reasoner.update_quads(
            store.quads_for_pattern(None, None, None, DefaultGraph())
        )
        closure = reasoner.reason_quads(graph_name=target)
        assert all(q.graph_name == target for q in closure)
        store.clear_graph(target)
        store.bulk_extend(closure)
        return removed

    assert refresh() is False
    assert Quad(ALICE, RDF_TYPE, AGENT, target) in store
    assert Quad(ALICE, RDF_TYPE, PERSON, target) in store
    assert Quad(BOB, RDF_TYPE, AGENT, target) not in store

    charlie = NamedNode("urn:charlie")
    store.add(Quad(charlie, RDF_TYPE, PERSON))
    assert refresh() is False
    assert Quad(charlie, RDF_TYPE, AGENT, target) in store

    store.remove(Quad(ALICE, RDF_TYPE, PERSON))
    assert refresh() is True
    assert Quad(ALICE, RDF_TYPE, AGENT, target) not in store
    assert Quad(charlie, RDF_TYPE, AGENT, target) in store


def test_named_source_and_default_output():
    source = NamedNode("urn:source")
    reasoner = PyReasoner()
    reasoner.add_quads([Quad(PERSON, SUBCLASS_OF, AGENT, source)])
    reasoner.add_quads([Triple(ALICE, RDF_TYPE, PERSON)])

    output = reasoner.reason_quads()
    assert Quad(ALICE, RDF_TYPE, AGENT) in output
    assert all(isinstance(q.graph_name, DefaultGraph) for q in output)


def test_blank_nodes_and_lexical_forms_round_trip():
    subject = BlankNode("subject")
    graph = BlankNode("target")
    predicate = NamedNode("urn:property")
    values = [
        NamedNode("urn:object"),
        BlankNode("object"),
        Literal("text"),
        Literal("bonjour", language="fr"),
        Literal("0042", datatype=NamedNode("http://www.w3.org/2001/XMLSchema#integer")),
    ]
    reasoner = PyReasoner()
    reasoner.add_quads(Triple(subject, predicate, value) for value in values)
    closure = reasoner.reason_quads(graph)
    assert {Quad(subject, predicate, value, graph) for value in values} <= set(closure)


def test_mixed_graphs_are_rejected_without_replacing_the_base():
    reasoner = PyReasoner()
    original = Quad(ALICE, RDF_TYPE, PERSON)
    reasoner.add_quads([original])

    with pytest.raises(ValueError, match="multiple graphs"):
        reasoner.update_quads(iter([
            Quad(PERSON, SUBCLASS_OF, AGENT),
            Quad(BOB, RDF_TYPE, PERSON, NamedNode("urn:other")),
        ]))

    closure = reasoner.reason_quads()
    assert original in closure
    assert Quad(ALICE, RDF_TYPE, AGENT) not in closure
    assert Quad(BOB, RDF_TYPE, PERSON) not in closure


@pytest.mark.parametrize(
    ("value", "message"),
    [
        (Triple(ALICE, RDF_TYPE, AGENT), "RDF-star triple terms"),
        (
            Literal("hello", language="en", direction=BaseDirection.LTR),
            "directional language-tagged literals",
        ),
    ],
)
def test_unsupported_terms_are_rejected_without_loading_partial_input(value, message):
    item = Quad(ALICE, RDF_TYPE, PERSON)
    reasoner = PyReasoner()
    with pytest.raises(TypeError, match=message):
        reasoner.add_quads(iter([item, Quad(ALICE, NamedNode("urn:property"), value)]))
    assert item not in reasoner.reason_quads()


def test_invalid_quad_or_output_graph_raises_type_error():
    reasoner = PyReasoner()
    with pytest.raises(TypeError, match="expected a pyoxigraph Quad or Triple"):
        reasoner.add_quads([(ALICE, RDF_TYPE, AGENT)])
    with pytest.raises(TypeError, match="graph_name"):
        reasoner.reason_quads("urn:graph")


def test_rdflib_api_works_without_pyoxigraph(monkeypatch):
    monkeypatch.setitem(sys.modules, "pyoxigraph", None)
    reasoner = PyReasoner()
    assert reasoner.reason()
    with pytest.raises(ModuleNotFoundError, match="pyoxigraph"):
        reasoner.reason_quads()
