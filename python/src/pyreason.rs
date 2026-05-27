use oxrdf::{BlankNode, Literal, NamedNode, Term, Triple};
use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
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

fn ox_term_from_py(s: &Bound<'_, PyAny>) -> PyResult<Term> {
    let typestr = s.get_type().name()?.to_string();
    match typestr.as_str() {
        "NamedNode" => {
            let v: String = s.getattr("value")?.extract()?;
            let n = NamedNode::new(v)
                .map_err(|e| exceptions::PyValueError::new_err(e.to_string()))?;
            Ok(Term::NamedNode(n))
        }
        "BlankNode" => {
            let v: String = s.getattr("value")?.extract()?;
            let b = BlankNode::new(v)
                .map_err(|e| exceptions::PyValueError::new_err(e.to_string()))?;
            Ok(Term::BlankNode(b))
        }
        "Literal" => {
            let v: String = s.getattr("value")?.extract()?;
            let lang: Option<String> = s.getattr("language")?.extract()?;
            if let Some(lang) = lang {
                let l = Literal::new_language_tagged_literal(v, lang)
                    .map_err(|e| exceptions::PyValueError::new_err(e.to_string()))?;
                Ok(Term::Literal(l))
            } else {
                let dt = s.getattr("datatype")?;
                let dt_value: String = dt.getattr("value")?.extract()?;
                let dt_nn = NamedNode::new(dt_value)
                    .map_err(|e| exceptions::PyValueError::new_err(e.to_string()))?;
                Ok(Term::Literal(Literal::new_typed_literal(v, dt_nn)))
            }
        }
        other => Err(exceptions::PyTypeError::new_err(format!(
            "expected pyoxigraph NamedNode/BlankNode/Literal, got {other}"
        ))),
    }
}

/// Extracts oxrdf Triples from an iterable of pyoxigraph Quad or Triple objects.
fn extract_triples_from_quads(py: Python, iterable: &PyObject) -> PyResult<Vec<Triple>> {
    let bound = iterable.bind(py);
    let iter = bound.try_iter()?;
    let mut triples: Vec<Triple> = Vec::new();
    for item in iter {
        let item = item?;
        let typestr = item.get_type().name()?.to_string();
        let (s_obj, p_obj, o_obj) = match typestr.as_str() {
            "Quad" => {
                let t = item.getattr("triple")?;
                (
                    t.getattr("subject")?,
                    t.getattr("predicate")?,
                    t.getattr("object")?,
                )
            }
            "Triple" => (
                item.getattr("subject")?,
                item.getattr("predicate")?,
                item.getattr("object")?,
            ),
            other => {
                return Err(exceptions::PyTypeError::new_err(format!(
                    "expected pyoxigraph Quad or Triple, got {other}"
                )));
            }
        };
        let s = ox_term_from_py(&s_obj)?;
        let p = ox_term_from_py(&p_obj)?;
        let o = ox_term_from_py(&o_obj)?;
        match make_triple(s, p, o) {
            Ok(triple) => triples.push(triple),
            Err(e) => return Err(PyReasoningError(e).into()),
        }
    }
    Ok(triples)
}

fn term_to_pyoxigraph<'a>(
    pyox: &Bound<'a, PyModule>,
    node: Term,
) -> PyResult<Bound<'a, PyAny>> {
    match node {
        Term::NamedNode(n) => pyox
            .getattr("NamedNode")?
            .call1((n.as_str().to_string(),)),
        Term::BlankNode(b) => pyox
            .getattr("BlankNode")?
            .call1((b.as_str().to_string(),)),
        Term::Literal(lit) => {
            let value = lit.value().to_string();
            let py = pyox.py();
            if let Some(lang) = lit.language() {
                let kwargs = PyDict::new(py);
                kwargs.set_item("language", lang)?;
                pyox.getattr("Literal")?.call((value,), Some(&kwargs))
            } else {
                let dt = lit.datatype().as_str().to_string();
                let nn = pyox.getattr("NamedNode")?.call1((dt,))?;
                let kwargs = PyDict::new(py);
                kwargs.set_item("datatype", nn)?;
                pyox.getattr("Literal")?.call((value,), Some(&kwargs))
            }
        }
    }
}

/// Converts oxrdf Triples into pyoxigraph Quads tagged with the given graph name
/// (or DefaultGraph if None).
fn triples_to_pyox_quads(
    py: Python,
    triples: Vec<Triple>,
    graph_name: Option<PyObject>,
) -> PyResult<Vec<PyObject>> {
    let pyox = py.import("pyoxigraph")?;
    let graph: Bound<'_, PyAny> = match graph_name {
        Some(g) => g.into_bound(py),
        None => pyox.getattr("DefaultGraph")?.call0()?,
    };
    let quad_cls = pyox.getattr("Quad")?;
    let mut res = Vec::new();
    for t in triples {
        let s = term_to_pyoxigraph(&pyox, Term::from(t.subject.clone()))?;
        let p = term_to_pyoxigraph(&pyox, Term::from(t.predicate.clone()))?;
        let o = term_to_pyoxigraph(&pyox, Term::from(t.object.clone()))?;
        let quad = quad_cls.call1((s, p, o, &graph))?;
        res.push(quad.into());
    }
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

    /// Appends triples drawn from an iterable of `pyoxigraph.Quad` or
    /// `pyoxigraph.Triple` objects. Quads' graph names are ignored — all triples
    /// are merged into the reasoner's single graph. Use `update_quads` to
    /// replace the base instead of appending.
    pub fn add_quads(&mut self, quads: PyObject) -> PyResult<()> {
        Python::with_gil(|py| {
            let triples = extract_triples_from_quads(py, &quads)?;
            self.reasoner.load_triples(triples);
            Ok(())
        })
    }

    /// Replaces the reasoner's base triples with those drawn from an iterable of
    /// `pyoxigraph.Quad` or `pyoxigraph.Triple` objects. Quads' graph names are
    /// ignored. Returns `True` if removals were detected (full re-materialization
    /// needed on the next `reason*` call), `False` otherwise.
    pub fn update_quads(&mut self, quads: PyObject) -> PyResult<bool> {
        Python::with_gil(|py| {
            let triples = extract_triples_from_quads(py, &quads)?;
            Ok(self.reasoner.set_base_triples(triples))
        })
    }

    /// Runs reasoning and returns the materialized triples as a list of
    /// `pyoxigraph.Quad`, tagged with `graph_name` (defaults to
    /// `pyoxigraph.DefaultGraph()`).
    #[pyo3(signature = (graph_name=None))]
    pub fn reason_quads(&mut self, graph_name: Option<PyObject>) -> PyResult<Vec<PyObject>> {
        Python::with_gil(|py| {
            self.reasoner.reason();
            triples_to_pyox_quads(py, self.reasoner.get_triples(), graph_name)
        })
    }
}
