use crate::owl;
use pyo3::prelude::*;
use pyo3::exceptions;

#[pyclass]
/// `PyReasoner` implements a reasoner for the OWL 2 RL profile (see
/// https://www.w3.org/TR/owl2-profiles/#OWL_2_RL for details).
struct PyReasoner {
    reasoner: owl::Reasoner,
}

#[pymethods]
impl PyReasoner {

    #[new]
    /// Creates a new PyReasoner object
    fn new() -> Self {
        PyReasoner{
            reasoner: owl::Reasoner::new()
        }
    }

    /// Loads in triples from an RDFlib Graph or any other object that can be converted into a list
    /// of triples (length-3 tuples of URI-formatted strings)
    pub fn from_graph(&mut self, graph: PyObject)  -> PyResult<()> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let converters = PyModule::from_code(py, "
def get_triples(graph):
    return list(graph)
", "converters.pg", "converters")?;
        let l: Vec<(String, String, String)> = converters.call1("get_triples", (graph,))?.extract()?;
        self.reasoner.load_triples(l);
        Ok(())
    }

    /// Loads in triples from a list
    pub fn load_triples(&mut self, triples: Vec<(&'static str, &'static str, &'static str)>) {
        self.reasoner.load_triples_str(triples);
    }

    /// Loads in triples from a file (turtle or n3 serialized)
    pub fn load_file(&mut self, file: String) -> PyResult<()> {
        // TODO: raise exception
        match self.reasoner.load_file(&file) {
            Ok(_) => Ok(()),
            Err(msg) => Err(exceptions::IOError::py_err(msg))
        }
    }

    /// Perform OWL 2 RL reasoning on the triples loaded into the PyReasoner object. This makes no
    /// assumptions about which ontologies are pre-loaded, so you need to load in OWL, RDFS, etc
    /// definitions in order to use them. Returns a list of triples
    pub fn reason(&mut self) -> PyResult<Vec<(String, String, String)>> {
        self.reasoner.reason();
        Ok(self.reasoner.get_triples())
    }
}

#[pymodule]
fn reasonable(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyReasoner>()?;
    Ok(())
}
