use crate::reasoner;
use crate::common::make_triple;
use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};
use oxigraph::model::{Term, NamedNode, BlankNode, Literal, Triple};
use std::convert::From;

// #[pyclass(name = "Reasoner", unsendable)]
#[pyclass(unsendable)]
/// `PyReasoner` implements a reasoner for the OWL 2 RL profile (see
/// https://www.w3.org/TR/owl2-profiles/#OWL_2_RL for details).
struct PyReasoner {
    reasoner: reasoner::Reasoner,
}

struct MyTerm(Term);
impl From<Result<&PyAny, pyo3::PyErr>> for MyTerm {
    fn from(s: Result<&PyAny, pyo3::PyErr>) -> Self {
        //if let Ok(typestr) = s.get_type().name() {
        let s = s.unwrap();
        let typestr = s.get_type().name().unwrap();
        let data_type: Option<NamedNode> = match s.getattr("datatype") {
            Ok(dt) => {
                if dt.is_none() {
                    None
                } else {
                    Some(NamedNode::new(dt.to_string()).unwrap())
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
        let n: Term = match typestr {
            "URIRef" => Term::NamedNode(NamedNode::new(s.to_string()).unwrap()),
            "Literal" => match (data_type, lang) {
                (Some(dt), None) => Term::Literal(Literal::new_typed_literal(s.to_string(), dt)),
                (None, Some(l)) => Term::Literal(Literal::new_language_tagged_literal(s.to_string(), l).unwrap()),
                (_, _) => Term::Literal(Literal::new_simple_literal(s.to_string())),
            },
            "BNode" => Term::BlankNode(BlankNode::new(s.to_string()).unwrap()),
            _ => Term::NamedNode(NamedNode::new(s.to_string()).unwrap()),
        };
       MyTerm(n)
    }
}

fn term_to_python<'a>(py: Python, rdflib: &'a PyModule, node: Term) -> PyResult<&'a PyAny> {
    let dtype: Option<String> = match &node {
        Term::Literal(lit) =>  {
            let mut s = lit.datatype().to_string();
            s.remove(0);
            s.remove(s.len()-1);
            Some(s)
        },
        _ => None,
    };
    let lang: Option<&str> = match &node {
        Term::Literal(lit) => lit.language(),
        _ => None,
    };

    let res: &PyAny = match &node {
        Term::NamedNode(uri) => {
            let mut uri = uri.to_string();
            uri.remove(0);
            uri.remove(uri.len()-1);
            rdflib.getattr("URIRef")?.call1((uri,))?
        },
        Term::Literal(literal) => {
            match (dtype, lang) {
                // prioritize 'lang' -> it implies String
                (_, Some(lang)) => rdflib.getattr("Literal")?.call1((literal.value(), lang, py.None()))?,
                (Some(dtype), None) => rdflib.getattr("Literal")?.call1((literal.value(), py.None(), dtype))?,
                (None, None) => rdflib.getattr("Literal")?.call1((literal.value(), ))?,
            }
        }
        Term::BlankNode(id) => {
            rdflib.getattr("BNode")?.call1((id.clone().into_string(),))?
        }
    };
    Ok(res)
}

//impl TryFrom<&PyAny> for MyTerm {
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
//            Ok(MyTerm(n))
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
        let l: &PyList = converters.getattr("get_triples")?.call1((graph,))?.downcast()?;
        let mut triples: Vec<Triple> = Vec::new();
        for t in l.iter() {
            let t: &PyTuple = t.downcast()?;
            let s = MyTerm::from(t.get_item(0)).0;
            let p = MyTerm::from(t.get_item(1)).0;
            let o = MyTerm::from(t.get_item(2)).0;
            match make_triple(s, p, o) {
                Ok(triple) => triples.push(triple),
                Err(e) => return Err(e.into())
            };
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
            let s = term_to_python(py, rdflib, Term::from(t.subject.clone()))?;
            let p = term_to_python(py, rdflib, Term::from(t.predicate.clone()))?;
            let o = term_to_python(py, rdflib, Term::from(t.object.clone()))?;
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
