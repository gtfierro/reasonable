use reasonable::common::make_triple;
use std::borrow::Borrow;
use pyo3_ffi::c_str;
use reasonable::error::ReasonableError;
use reasonable::reasoner;
use oxrdf::{BlankNode, Literal, NamedNode, Term, Triple};
use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};
use std::convert::From;

struct PyReasoningError(ReasonableError);

impl std::convert::From<PyReasoningError> for PyErr {
    fn from(err: PyReasoningError) -> PyErr {
        let PyReasoningError(err) = err;
        exceptions::PyException::new_err(err.to_string())
    }
}

#[allow(dead_code)]
struct MyTerm(Term);
impl From<Result<Bound<'_, PyAny>, pyo3::PyErr>> for MyTerm {
    fn from(s: Result<Bound<'_, PyAny>, pyo3::PyErr>) -> Self {
        let s = s.unwrap();
        let typestr = s.get_type().name().unwrap();
        let typestr = typestr.to_string();
        let data_type: Option<NamedNode> = match s.getattr("datatype") {
            Ok(dt) => {
                if dt.is_none() {
                    None
                } else {
                    Some(NamedNode::new(dt.to_string()).unwrap())
                }
            }
            Err(_) => None,
        };
        let lang: Option<String> = match s.getattr("language") {
            Ok(l) => {
                if l.is_none() {
                    None
                } else {
                    Some(l.to_string())
                }
            }
            Err(_) => None,
        };
        let n: Term = match typestr.borrow() {
            "URIRef" => Term::NamedNode(NamedNode::new(s.to_string()).unwrap()),
            "Literal" => match (data_type, lang) {
                (Some(dt), None) => Term::Literal(Literal::new_typed_literal(s.to_string(), dt)),
                (None, Some(l)) => {
                    Term::Literal(Literal::new_language_tagged_literal(s.to_string(), l).unwrap())
                }
                (_, _) => Term::Literal(Literal::new_simple_literal(s.to_string())),
            },
            "BNode" => Term::BlankNode(BlankNode::new(s.to_string()).unwrap()),
            _ => Term::NamedNode(NamedNode::new(s.to_string()).unwrap()),
        };
        MyTerm(n)
    }
}

fn term_to_python<'a>(
    py: Python,
    rdflib: &Bound<'a, PyModule>,
    node: Term,
) -> PyResult<Bound<'a, PyAny>> {
    let dtype: Option<String> = match &node {
        Term::Literal(lit) => {
            let mut s = lit.datatype().to_string();
            s.remove(0);
            s.remove(s.len() - 1);
            Some(s)
        }
        _ => None,
    };
    let lang: Option<&str> = match &node {
        Term::Literal(lit) => lit.language(),
        _ => None,
    };

    let res: Bound<'_, PyAny> = match &node {
        Term::NamedNode(uri) => {
            let mut uri = uri.to_string();
            uri.remove(0);
            uri.remove(uri.len() - 1);
            rdflib.getattr("URIRef")?.call1((uri,))?
        }
        Term::Literal(literal) => {
            match (dtype, lang) {
                // prioritize 'lang' -> it implies String
                (_, Some(lang)) => {
                    rdflib
                        .getattr("Literal")?
                        .call1((literal.value(), lang, py.None()))?
                }
                (Some(dtype), None) => {
                    rdflib
                        .getattr("Literal")?
                        .call1((literal.value(), py.None(), dtype))?
                }
                (None, None) => rdflib.getattr("Literal")?.call1((literal.value(),))?,
            }
        }
        Term::BlankNode(id) => rdflib
            .getattr("BNode")?
            .call1((id.clone().into_string(),))?,
    };
    Ok(res)
}

#[pyclass(unsendable)]
pub struct PyReasoner {
    reasoner: reasoner::Reasoner,
}

#[pymethods]
impl PyReasoner {
    #[new]
    /// Creates a new PyReasoner object
    fn new() -> Self {
        PyReasoner {
            reasoner: reasoner::Reasoner::new(),
        }
    }

    /// Loads in triples from a file (turtle or n3 serialized)
    pub fn load_file(&mut self, file: String) -> PyResult<()> {
        match self.reasoner.load_file(&file) {
            Ok(_) => Ok(()),
            Err(msg) => Err(exceptions::PyIOError::new_err(msg)),
        }
    }

    /// Perform OWL 2 RL reasoning on the triples loaded into the PyReasoner object. This makes no
    /// assumptions about which ontologies are pre-loaded, so you need to load in OWL, RDFS, etc
    /// definitions in order to use them. Returns a list of triples
    pub fn reason(&mut self) -> PyResult<Vec<(Py<PyAny>, Py<PyAny>, Py<PyAny>)>> {
        Python::with_gil(|py| {
            let rdflib = py.import("rdflib")?;
            self.reasoner.reason();
            let mut res = Vec::new();
            for t in self.reasoner.get_triples() {
                let s = term_to_python(py, &rdflib, Term::from(t.subject.clone()))?;
                let p = term_to_python(py, &rdflib, Term::from(t.predicate.clone()))?;
                let o = term_to_python(py, &rdflib, Term::from(t.object.clone()))?;
                res.push((s.into(), p.into(), o.into()));
            }
            Ok(res)
        })
    }

    /// Loads in triples from an RDFlib Graph or any other object that can be converted into a list
    /// of triples (length-3 tuples of URI-formatted strings)
    pub fn from_graph(&mut self, graph: PyObject) -> PyResult<()> {
        Python::with_gil(|py| {
            let converters = PyModule::from_code(
                py,
                c_str!("def get_triples(graph):
    return list(graph)
"),
                c_str!("converters.pg"),
                c_str!("converters"),
            )?;
            let binding = converters.getattr("get_triples")?.call1((graph,))?;
            let l: &Bound<'_, PyList> = binding.downcast()?;
            let mut triples: Vec<Triple> = Vec::new();
            for t in l.iter() {
                let t: &Bound<'_, PyTuple> = t.downcast()?;
                let s = MyTerm::from(t.get_item(0)).0;
                let p = MyTerm::from(t.get_item(1)).0;
                let o = MyTerm::from(t.get_item(2)).0;
                match make_triple(s, p, o) {
                    Ok(triple) => triples.push(triple),
                    Err(e) => return Err(PyReasoningError(e).into()),
                };
            }
            self.reasoner.load_triples(triples);
            Ok(())
        })
    }
}
