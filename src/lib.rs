//! The `reasonable` package offers a library, binary and Python bindings for performing OWL 2 RL
//! reasoning on RDF graphs. This package implements the Datalog rules as communicated on the [W3C
//! OWL2
//! Profile](https://www.w3.org/TR/owl2-profiles/#Reasoning_in_OWL_2_RL_and_RDF_Graphs_using_Rules)
//! website.
// #[macro_use] extern crate ketos;

#[allow(dead_code)]
mod index;
#[allow(dead_code)]
pub mod reasoner;
//#[allow(dead_code)]
//pub mod query;
//#[allow(dead_code)]
//pub mod algebra;
#[allow(dead_code)]
pub mod manager;
pub mod error;

mod disjoint_sets;
#[allow(dead_code)]
#[allow(unused_macros)]
mod common;

#[cfg(feature="python-library")]
mod python;

#[cfg(test)]
mod tests;


