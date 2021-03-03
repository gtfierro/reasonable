use crate::reasoner;
use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};
use rdf::node::Node;
use rdf::uri::Uri;

#[pyclass(name="Reasoner", unsendable)]
/// `PyReasoner` implements a reasoner for the OWL 2 RL profile (see
/// https://www.w3.org/TR/owl2-profiles/#OWL_2_RL for details).
struct PyReasoner {
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

    /// Loads in triples from an RDFlib Graph or any other object that can be converted into a list
    /// of triples (length-3 tuples of URI-formatted strings)
    pub fn from_graph(&mut self, graph: PyObject) -> PyResult<()> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let converters = PyModule::from_code(
            py,
            "
def get_triples(graph):
    return list(graph)
",
            "converters.pg",
            "converters",
        )?;
        let l: &PyList = converters.call1("get_triples", (graph,))?.downcast()?;
        let mut triples: Vec<(Node, Node, Node)> = Vec::new();
        for t in l.iter() {
            let t: &PyTuple = t.downcast()?;
            let _s: &PyAny = t.get_item(0);
            let _p: &PyAny = t.get_item(1);
            let _o: &PyAny = t.get_item(2);

            let s: Node = {
                if let Ok(typestr) = _s.get_type().name() {
                    match typestr {
                        "URIRef" => Node::UriNode {
                            uri: Uri::new(_s.to_string()),
                        },
                        "Literal" => Node::LiteralNode {
                            literal: _s.to_string(),
                            data_type: Some(Uri::new(_s.getattr("datatype")?.to_string())),
                            language: None,
                        },
                        "BNode" => Node::BlankNode { id: _s.to_string() },
                        _ => Node::UriNode {
                            uri: Uri::new(_s.to_string()),
                        },
                    }
                } else {
                    continue
                }
            };

            let p: Node = {
                if let Ok(typestr) = _p.get_type().name() {
                    match typestr {
                        "URIRef" => Node::UriNode {
                            uri: Uri::new(_p.to_string()),
                        },
                        "Literal" => Node::LiteralNode {
                            literal: _p.to_string(),
                            data_type: Some(Uri::new(_p.getattr("datatype")?.to_string())),
                            language: None,
                        },
                        "BNode" => Node::BlankNode { id: _p.to_string() },
                        _ => Node::UriNode {
                            uri: Uri::new(_p.to_string()),
                        },
                    }
                } else {
                    continue
                }
            };
            let o: Node = {
                if let Ok(typestr) = _o.get_type().name() {
                    match typestr {
                        "URIRef" => Node::UriNode {
                            uri: Uri::new(_o.to_string()),
                        },
                        "Literal" => Node::LiteralNode {
                            literal: _o.to_string(),
                            data_type: Some(Uri::new(_o.getattr("datatype")?.to_string())),
                            language: None,
                        },
                        "BNode" => Node::BlankNode { id: _o.to_string() },
                        _ => Node::UriNode {
                            uri: Uri::new(_o.to_string()),
                        },
                    }
                } else {
                    continue
                }
            };
            triples.push((s, p, o));
        }
        //let l: Vec<(PyObject, PyObject, PyObject)> = converters.call1("get_triples", (graph,))?.extract()?;
        //for t in l {
        //    println!("{:?}", t.0.get_type());
        //}
        self.reasoner.load_triples(triples);
        Ok(())
    }

    // /// Loads in triples from a list
    // pub fn load_triples(&mut self, triples: Vec<(&'static str, &'static str, &'static str)>) -> PyResult<()> {
    //     self.reasoner.load_triples_str(triples);
    //     Ok(())
    // }

    /// Loads in triples from a file (turtle or n3 serialized)
    pub fn load_file(&mut self, file: String) -> PyResult<()> {
        // TODO: raise exception
        match self.reasoner.load_file(&file) {
            Ok(_) => Ok(()),
            //Err(msg) => Err(exceptions::IOError::py_err(msg)),
            Err(msg) => Err(exceptions::PyIOError::new_err(msg)),
        }
    }

    /// Perform OWL 2 RL reasoning on the triples loaded into the PyReasoner object. This makes no
    /// assumptions about which ontologies are pre-loaded, so you need to load in OWL, RDFS, etc
    /// definitions in order to use them. Returns a list of triples
    pub fn reason(&mut self) -> PyResult<Vec<(Py<PyAny>, Py<PyAny>, Py<PyAny>)>> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let rdflib = py.import("rdflib")?;

        self.reasoner.reason();
        let mut res = Vec::new();
        for t in self.reasoner.get_triples() {
            let s = match &t.0 {
                Node::UriNode { ref uri } => rdflib.call1("URIRef", (uri.to_string(),))?,
                Node::LiteralNode {
                    ref literal,
                    data_type: None,
                    language: _,
                } => rdflib.call1("Literal", (literal.to_string(), ))?,
                Node::LiteralNode {
                    ref literal,
                    data_type: Some(ref dtype),
                    language: _,
                } => rdflib.call1("Literal", (literal.to_string(), py.None(), dtype.to_string()))?,
                Node::BlankNode { ref id } => rdflib.call1("BNode", (id.to_string(),))?,
            };
            let p = match &t.1 {
                Node::UriNode { ref uri } => rdflib.call1("URIRef", (uri.to_string(),))?,
                Node::LiteralNode {
                    ref literal,
                    data_type: None,
                    language: _,
                } => rdflib.call1("Literal", (literal.to_string(), ))?,
                Node::LiteralNode {
                    ref literal,
                    data_type: Some(ref dtype),
                    language: _,
                } => rdflib.call1("Literal", (literal.to_string(), py.None(), dtype.to_string()))?,
                Node::BlankNode { ref id } => rdflib.call1("BNode", (id.to_string(),))?,
            };
            let o = match &t.2 {
                Node::UriNode { ref uri } => rdflib.call1("URIRef", (uri.to_string(),))?,
                Node::LiteralNode {
                    ref literal,
                    data_type: None,
                    language: _,
                } => rdflib.call1("Literal", (literal.to_string(), ))?,
                Node::LiteralNode {
                    ref literal,
                    data_type: Some(ref dtype),
                    language: _,
                } => rdflib.call1("Literal", (literal.to_string(), py.None(), dtype.to_string()))?,
                Node::BlankNode { ref id } => rdflib.call1("BNode", (id.to_string(),))?,
            };
            res.push((s.into(), p.into(), o.into()));
        }
        Ok(res)
    }
}

#[pymodule]
fn reasonable(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyReasoner>()?;
    Ok(())
}
