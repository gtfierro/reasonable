extern crate datafrog;
use datafrog::{Iteration, Variable};

use crate::index::URIIndex;
use crate::types::{URI, has_pred, has_obj, has_pred_obj};

pub struct Reasoner {
    iter1: Iteration,
    index: URIIndex,

    spo: Variable<(URI, (URI, URI))>,
    pso: Variable<(URI, (URI, URI))>,
    osp: Variable<(URI, (URI, URI))>,
    all_triples_input: Variable<(URI, (URI, URI))>,
}

impl Reasoner {
    pub fn new() -> Self {
        let mut iter1 = Iteration::new();
        let mut index = URIIndex::new();

        // variables within the iteration
        let spo = iter1.variable::<(URI, (URI, URI))>("spo");
        let pso = iter1.variable::<(URI, (URI, URI))>("pso");
        let osp = iter1.variable::<(URI, (URI, URI))>("osp");
        let all_triples_input = iter1.variable::<(URI, (URI, URI))>("all_triples_input");

        Reasoner {
            iter1: iter1,
            index: index,
            spo: spo,
            pso: pso,
            osp: osp,
            all_triples_input: all_triples_input,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_reasoner() -> Result<(), String> {
        Ok(())
    }
}
