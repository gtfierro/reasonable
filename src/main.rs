extern crate datafrog;
extern crate rdf;
use datafrog::Iteration;

mod index;
mod types;
mod owl;
use crate::index::URIIndex;
use crate::types::{URI, has_pred, has_obj, has_pred_obj};
use crate::owl::Reasoner;

use std::fs;

use rdf::reader::turtle_parser::TurtleParser;
use rdf::reader::n_triples_parser::NTriplesParser;
use rdf::reader::rdf_parser::RdfParser;
use rdf::node::Node;
use rdf::graph::Graph;

// TODO: implement lists; how do we translate this?

fn main() {

    let mut r = Reasoner::new();

    // TODO: load in datasets
    r.load_file("rdfs.ttl");
    // Brick.ttl has some parse error so we use n3
    r.load_file("Brick.n3");
    r.load_file("example.n3");

    r.reason();

    r.dump();

//    let v1 : Vec::<(URI, (URI, URI))> = vec![
//        (index.put_str("a"), (index.put_str("rdf:type"), index.put_str("Class1"))),
//        (index.put_str("b"), (index.put_str("rdf:type"), index.put_str("Class1"))),
//        (index.put_str("Class1"), (index.put_str("rdfs:subClassOf"), index.put_str("Class2"))),
//        (index.put_str("Class3"), (index.put_str("rdfs:subClassOf"), index.put_str("Class2"))),
//        (index.put_str("brick:feeds"), (index.put_str("rdfs:domain"), index.put_str("Class2"))),
//        (index.put_str("brick:feeds"), (index.put_str("rdfs:range"), index.put_str("Class3"))),
//        (index.put_str("brick:isFedBy"), (index.put_str("owl:inverseOf"), index.put_str("brick:feeds"))),
//        (index.put_str("c"), (index.put_str("brick:feeds"), index.put_str("d"))),
//
//        // cls-thing
//        (index.put_str("owl:Thing"), (index.put_str("rdf:type"), index.put_str("owl:Class"))),
//        // cls-nothing1
//        (index.put_str("owl:Nothing"), (index.put_str("rdf:type"), index.put_str("owl:Class"))),
//
//        // owl definitions
//        (index.put_str("owl:inverseOf"), (index.put_str("rdf:type"), index.put_str("owl:SymmetricProperty"))),
//    ];
    //all_triples_input.insert(v1.into());


}
