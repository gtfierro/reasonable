use oxrdf::{BlankNode, Literal, NamedNode, Term, TermRef, Triple};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use reasonable::common::make_triple;

pub(crate) struct Pyoxigraph<'py> {
    named_node: Bound<'py, PyAny>,
    blank_node: Bound<'py, PyAny>,
    literal: Bound<'py, PyAny>,
    triple: Bound<'py, PyAny>,
    quad: Bound<'py, PyAny>,
    default_graph: Bound<'py, PyAny>,
}

impl<'py> Pyoxigraph<'py> {
    pub(crate) fn import(py: Python<'py>) -> PyResult<Self> {
        let model = py.import("pyoxigraph")?;
        Ok(Self {
            named_node: model.getattr("NamedNode")?,
            blank_node: model.getattr("BlankNode")?,
            literal: model.getattr("Literal")?,
            triple: model.getattr("Triple")?,
            quad: model.getattr("Quad")?,
            default_graph: model.getattr("DefaultGraph")?,
        })
    }

    pub(crate) fn extract_triples(&self, items: &Bound<'py, PyAny>) -> PyResult<Vec<Triple>> {
        let mut source_graph: Option<Bound<'py, PyAny>> = None;
        items
            .try_iter()?
            .map(|item| {
                let item = item?;
                if item.is_instance(&self.quad)? {
                    let graph = item.getattr("graph_name")?;
                    if let Some(source) = &source_graph {
                        if !source.eq(&graph)? {
                            return Err(PyValueError::new_err(
                                "quads belong to multiple graphs; select one graph with quads_for_pattern or pass Triple objects to reason over their union",
                            ));
                        }
                    } else {
                        source_graph = Some(graph);
                    }
                } else if !item.is_instance(&self.triple)? {
                    return Err(PyTypeError::new_err("expected a pyoxigraph Quad or Triple"));
                }
                let subject = self.extract_term(&item.getattr("subject")?)?;
                let predicate = self.extract_term(&item.getattr("predicate")?)?;
                let object = self.extract_term(&item.getattr("object")?)?;
                make_triple(subject, predicate, object)
                    .map_err(|err| PyTypeError::new_err(err.to_string()))
            })
            .collect()
    }

    fn extract_term(&self, term: &Bound<'py, PyAny>) -> PyResult<Term> {
        if term.is_instance(&self.named_node)? {
            let value: String = term.getattr("value")?.extract()?;
            return NamedNode::new(value).map(Term::from).map_err(value_error);
        }
        if term.is_instance(&self.blank_node)? {
            let value: String = term.getattr("value")?.extract()?;
            return BlankNode::new(value).map(Term::from).map_err(value_error);
        }
        if term.is_instance(&self.literal)? {
            if !term.getattr("direction")?.is_none() {
                return Err(PyTypeError::new_err(
                    "Reasonable does not support RDF 1.2 directional language-tagged literals",
                ));
            }
            let value: String = term.getattr("value")?.extract()?;
            let language: Option<String> = term.getattr("language")?.extract()?;
            let literal = if let Some(language) = language {
                Literal::new_language_tagged_literal(value, language).map_err(value_error)?
            } else {
                let datatype: String = term.getattr("datatype")?.getattr("value")?.extract()?;
                Literal::new_typed_literal(value, NamedNode::new(datatype).map_err(value_error)?)
            };
            return Ok(literal.into());
        }
        if term.is_instance(&self.triple)? {
            return Err(PyTypeError::new_err(
                "Reasonable does not support RDF-star triple terms",
            ));
        }
        Err(PyTypeError::new_err("expected a pyoxigraph RDF term"))
    }

    pub(crate) fn output_graph(
        &self,
        graph_name: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let Some(graph) = graph_name else {
            return self.default_graph.call0();
        };
        if graph.is_instance(&self.named_node)?
            || graph.is_instance(&self.blank_node)?
            || graph.is_instance(&self.default_graph)?
        {
            return Ok(graph);
        }
        Err(PyTypeError::new_err(
            "graph_name must be a pyoxigraph NamedNode, BlankNode, or DefaultGraph",
        ))
    }

    pub(crate) fn make_quads(
        &self,
        triples: &[Triple],
        graph: &Bound<'py, PyAny>,
    ) -> PyResult<Vec<Bound<'py, PyAny>>> {
        triples
            .iter()
            .map(|triple| {
                let subject = self.make_term(triple.subject.as_ref().into())?;
                let predicate = self.named_node.call1((triple.predicate.as_str(),))?;
                let object = self.make_term(triple.object.as_ref())?;
                self.quad.call1((subject, predicate, object, graph))
            })
            .collect()
    }

    fn make_term(&self, term: TermRef<'_>) -> PyResult<Bound<'py, PyAny>> {
        match term {
            TermRef::NamedNode(node) => self.named_node.call1((node.as_str(),)),
            TermRef::BlankNode(node) => self.blank_node.call1((node.as_str(),)),
            TermRef::Literal(literal) => {
                let kwargs = PyDict::new(self.literal.py());
                if let Some(language) = literal.language() {
                    kwargs.set_item("language", language)?;
                } else {
                    let datatype = self.named_node.call1((literal.datatype().as_str(),))?;
                    kwargs.set_item("datatype", datatype)?;
                }
                self.literal.call((literal.value(),), Some(&kwargs))
            }
        }
    }
}

fn value_error(error: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(error.to_string())
}
