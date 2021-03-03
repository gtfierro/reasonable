use crate::reasoner;
use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};
use rdf::node::Node;
use rdf::uri::Uri;
use std::convert::From;

// #[pyclass(name = "Reasoner", unsendable)]
#[pyclass(unsendable)]
/// `PyReasoner` implements a reasoner for the OWL 2 RL profile (see
/// https://www.w3.org/TR/owl2-profiles/#OWL_2_RL for details).
struct PyReasoner {
    reasoner: reasoner::Reasoner,
}

struct MyNode(Node);
impl From<&PyAny> for MyNode {
    fn from(s: &PyAny) -> Self {
        //if let Ok(typestr) = s.get_type().name() {
        let typestr = s.get_type().name().unwrap();
        let data_type: Option<Uri> = match s.getattr("datatype") {
            Ok(dt) => {
                if dt.is_none() {
                    None
                } else {
                    Some(Uri::new(dt.to_string()))
                }
            },
            Err(_) => None,
        };
        let lang: Option<String> = match s.getattr("language") {
            Ok(l) => {
                if l.is_none() {
                    None
                } else {
                    Some(l.to_string())
                }
            },
            Err(_) => None,
        };
        let n: Node = match typestr {
            "URIRef" => Node::UriNode {
                uri: Uri::new(s.to_string()),
            },
            "Literal" => Node::LiteralNode {
                literal: s.to_string(),
                data_type: data_type,
                language: lang,
            },
            "BNode" => Node::BlankNode { id: s.to_string() },
            _ => Node::UriNode {
                uri: Uri::new(s.to_string()),
            },
        };
       MyNode(n)
    }
}

fn node_to_python<'a>(py: Python, rdflib: &'a PyModule, node: &'a Node) -> PyResult<&'a PyAny> {
    let dtype: Option<&String> = match node {
        Node::LiteralNode{literal: _, data_type: Some(dt), language: _} =>  Some(dt.to_string()),
        _ => None,
    };
    let lang: Option<&String> = match node {
        Node::LiteralNode{literal: _, data_type: _, language: Some(l)} => Some(&l),
        _ => None,
    };

    let res: &PyAny = match node {
        Node::UriNode { ref uri } => rdflib.call1("URIRef", (uri.to_string(),))?,
        Node::LiteralNode {
            ref literal,
            data_type: _,
            language: _,
        } => {
            match (dtype, lang) {
                (Some(dtype), Some(lang)) => rdflib.call1("Literal", (literal.to_string(), lang, dtype))?,
                (None, Some(lang)) => rdflib.call1("Literal", (literal.to_string(), lang, py.None()))?,
                (Some(dtype), None) => rdflib.call1("Literal", (literal.to_string(), py.None(), dtype))?,
                (None, None) => rdflib.call1("Literal", (literal.to_string(), ))?,
            }
        }
        Node::BlankNode { ref id } => rdflib.call1("BNode", (id.to_string(),))?,
    };
    Ok(res)
}

//impl TryFrom<&PyAny> for MyNode {
//    type Error = PyErr;
//    fn try_from(s: &PyAny) -> Result<Self, Self::Error> {
//        if let Ok(typestr) = s.get_type().name() {
//            let data_type: Option<Uri> = match s.getattr("datatype") {
//                Ok(dt) => Some(Uri::new(dt.to_string())),
//                Err(_) => None,
//            };
//            let n: Node = match typestr {
//                "URIRef" => Node::UriNode {
//                    uri: Uri::new(s.to_string()),
//                },
//                "Literal" => Node::LiteralNode {
//                    literal: s.to_string(),
//                    // TODO: sometimes this can be "None", which is incorrect -> make sure
//                    // it is None when passed to Rust
//                    data_type: data_type,
//                    // TODO: handle language
//                    language: None,
//                },
//                "BNode" => Node::BlankNode { id: s.to_string() },
//                _ => Node::UriNode {
//                    uri: Uri::new(s.to_string()),
//                },
//            };
//            Ok(MyNode(n))
//        } else {
//            Err(exceptions::PyValueError::new_err("No type available"))
//        }
//    }
//}

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
            let s = MyNode::from(t.get_item(0)).0;
            let p = MyNode::from(t.get_item(1)).0;
            let o = MyNode::from(t.get_item(2)).0;
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
        match self.reasoner.load_file(&file) {
            Ok(_) => Ok(()),
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
            let s = node_to_python(py, rdflib, &t.0)?;
            let p = node_to_python(py, rdflib, &t.1)?;
            let o = node_to_python(py, rdflib, &t.2)?;
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
