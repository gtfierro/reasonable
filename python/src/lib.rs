mod pyreason;
use pyo3::prelude::*;

#[pymodule]
fn pyoxigraph(_py: Python<'_>, module: &PyModule) -> PyResult<()> {
    module.add("__package__", "pyreasonable")?;
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    module.add_class::<pyreason::PyReasoner>()?;
    Ok(())
}
