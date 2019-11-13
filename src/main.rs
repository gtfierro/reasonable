extern crate rdf;

mod index;
mod types;
mod owl;
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
    r.load_file("rdfs.ttl").unwrap();
    // Brick.ttl has some parse error so we use n3
    r.load_file("Brick.n3").unwrap();
    r.load_file("example.n3").unwrap();

    let v1 : Vec::<(&str, &str, &str)> = vec![
        ("a", "rdf:type", "Class1"),
        ("b", "rdf:type", "Class1"),
        ("Class1", "rdfs:subClassOf", "Class2"),
        ("Class3", "rdfs:subClassOf", "Class2"),
        ("brick:feeds", "rdfs:domain", "Class2"),
        ("brick:feeds", "rdfs:range", "Class3"),
        ("brick:isFedBy", "owl:inverseOf", "brick:feeds"),
        ("c", "brick:feeds", "d"),

        // cls-thing
        ("owl:Thing", "rdf:type", "owl:Class"),
        // cls-nothing1
        ("owl:Nothing", "rdf:type", "owl:Class"),

        // owl definitions
        ("owl:inverseOf", "rdf:type", "owl:SymmetricProperty"),
    ];
    r.load_triples(v1);

    r.reason();

    //for i in r.get_triples() {
    //    let (s, p, o) = i;
    //    println!("> {} {} {}", s, p, o);
    //}
}
