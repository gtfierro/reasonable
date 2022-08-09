mod utils;

use reasonable::reasoner;
use oxigraph::io::{GraphFormat, GraphParser};
use std::io::Cursor;
use oxigraph::model::Triple;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub struct Context {
    reasoner: reasoner::Reasoner
}

impl Context {
    fn new() -> Self {
        Context { reasoner: reasoner::Reasoner::new() }
    }

    fn load_graph(&mut self, graph_turtle: String) {
        let parser = GraphParser::from_format(GraphFormat::NTriples);
        let triples = parser.read_triples(Cursor::new(graph_turtle)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        self.reasoner.load_triples(triples);
    }

    fn reason(&mut self) {
        self.reasoner.reason();
    }

    fn get_triples(&self) -> &[Triple] {
        self.reasoner.view_output()
    }
}
