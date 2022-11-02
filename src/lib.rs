//! The `reasonable` package offers a library, binary and Python bindings for performing OWL 2 RL
//! reasoning on RDF graphs. This package implements the Datalog rules as communicated on the [W3C
//! OWL2
//! Profile](https://www.w3.org/TR/owl2-profiles/#Reasoning_in_OWL_2_RL_and_RDF_Graphs_using_Rules)
//! website.
#[macro_use]
#[allow(dead_code, unused_macros)]
pub mod common;
mod disjoint_sets;
pub mod error;
#[cfg(feature = "ondisk")]
#[allow(dead_code)]
pub mod graphmanager;
#[allow(dead_code)]
mod index;
#[cfg(feature = "ondisk")]
#[allow(dead_code)]
pub mod manager;
#[allow(dead_code)]
pub mod reasoner;

#[cfg(test)]
mod tests;

#[cfg(feature = "python")]
mod pyreason;
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
#[pymodule]
fn reasonable(_py: Python<'_>, module: &PyModule) -> PyResult<()> {
    module.add("__package__", "reasonable")?;
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    module.add_class::<pyreason::PyReasoner>()?;
    Ok(())
}
