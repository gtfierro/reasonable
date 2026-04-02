use oxrdf::{BlankNode, Literal, NamedNode, Term, Triple};
use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};
use pyo3_ffi::c_str;
use reasonable::common::make_triple;
use reasonable::error::ReasonableError;
use reasonable::reasoner;
use std::borrow::Borrow;
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

/// Converts the output of reason() or get_triples() to Python rdflib terms.
fn triples_to_python(
    py: Python,
    triples: Vec<Triple>,
) -> PyResult<Vec<(Py<PyAny>, Py<PyAny>, Py<PyAny>)>> {
    let rdflib = py.import("rdflib")?;
    let mut res = Vec::new();
    for t in triples {
        let s = term_to_python(py, &rdflib, Term::from(t.subject.clone()))?;
        let p = term_to_python(py, &rdflib, Term::from(t.predicate.clone()))?;
        let o = term_to_python(py, &rdflib, Term::from(t.object.clone()))?;
        res.push((s.into(), p.into(), o.into()));
    }
    Ok(res)
}

/// Extracts triples from an rdflib Graph (or any iterable of 3-tuples) into oxrdf Triples.
fn extract_triples_from_graph(py: Python, graph: &PyObject) -> PyResult<Vec<Triple>> {
    let converters = PyModule::from_code(
        py,
        c_str!(
            "def get_triples(graph):
    return list(graph)
"
        ),
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
    Ok(triples)
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
            Err(err) => Err(exceptions::PyIOError::new_err(err.to_string())),
        }
    }

    /// Perform OWL 2 RL reasoning on the triples loaded into the PyReasoner object. This makes no
    /// assumptions about which ontologies are pre-loaded, so you need to load in OWL, RDFS, etc
    /// definitions in order to use them. Returns a list of triples
    pub fn reason(&mut self) -> PyResult<Vec<(Py<PyAny>, Py<PyAny>, Py<PyAny>)>> {
        Python::with_gil(|py| {
            self.reasoner.reason();
            triples_to_python(py, self.reasoner.get_triples())
        })
    }

    /// Loads in triples from an RDFlib Graph or any other object that can be converted into a list
    /// of triples (length-3 tuples of URI-formatted strings).
    ///
    /// Note: this *appends* to the existing base triples. To *replace* the base
    /// (supporting retraction), use `update_graph()` instead.
    pub fn from_graph(&mut self, graph: PyObject) -> PyResult<()> {
        Python::with_gil(|py| {
            let triples = extract_triples_from_graph(py, &graph)?;
            self.reasoner.load_triples(triples);
            Ok(())
        })
    }

    /// Replaces the reasoner's base triples with the contents of the given graph.
    ///
    /// Computes a diff against the current base:
    /// - If only additions are detected, the next `reason()` call uses incremental materialization.
    /// - If any removals are detected, the next `reason()` call performs a full re-materialization.
    ///
    /// Returns `True` if removals were detected (full re-materialization needed),
    /// `False` if only additions (or no change).
    pub fn update_graph(&mut self, graph: PyObject) -> PyResult<bool> {
        Python::with_gil(|py| {
            let triples = extract_triples_from_graph(py, &graph)?;
            let needs_full = self.reasoner.set_base_triples(triples);
            Ok(needs_full)
        })
    }

    /// Clears all inferred state, resetting to base triples.
    /// The next `reason()` call will perform a full re-materialization.
    pub fn clear(&mut self) {
        self.reasoner.clear();
    }

    /// Forces a full re-materialization from base triples.
    /// Equivalent to `clear()` followed by `reason()`.
    /// Returns a list of all triples (base + inferred).
    pub fn reason_full(&mut self) -> PyResult<Vec<(Py<PyAny>, Py<PyAny>, Py<PyAny>)>> {
        Python::with_gil(|py| {
            self.reasoner.reason_full();
            triples_to_python(py, self.reasoner.get_triples())
        })
    }

    /// Returns the current base (non-inferred) triples as rdflib terms.
    pub fn get_base_triples(&self) -> PyResult<Vec<(Py<PyAny>, Py<PyAny>, Py<PyAny>)>> {
        Python::with_gil(|py| {
            triples_to_python(py, self.reasoner.get_input())
        })
    }
}
