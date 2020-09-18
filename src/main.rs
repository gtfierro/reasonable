use ::reasonable::owl::Reasoner;
use std::env;
use std::time::Instant;
use log::info;

use rdf::node::Node;
use oxigraph::MemoryStore;
use oxigraph::model::*;
use oxigraph::sparql::{QueryOptions, QueryResults};

fn main() {
    env_logger::init();
    let mut r = Reasoner::new();
    env::args().skip(1).map(|filename| {
        info!("Loading file {}", &filename);
        r.load_file(&filename).unwrap()
    }).count();
    let reasoning_start = Instant::now();
    info!("Starting reasoning");
    r.reason();
    info!("Reasoning completed in {:.02}sec", reasoning_start.elapsed().as_secs_f64());

    let store = MemoryStore::new();
    for t in r.get_triples() {
        let s = match t.0 {
            Node::UriNode{uri} => NamedOrBlankNode::NamedNode(NamedNode::new_unchecked(uri.to_string())),
            Node::BlankNode{id} => NamedOrBlankNode::BlankNode(BlankNode::new_unchecked(id.to_string())),
            _ => panic!("no subject literals"),
        };
        let p = match t.1 {
            Node::UriNode{uri} => NamedNode::new_unchecked(uri.to_string()),
            _ => panic!("no must be named node"),
        };
        let o = match t.2 {
            Node::UriNode{uri} => Term::NamedNode(NamedNode::new_unchecked(uri.to_string())),
            Node::BlankNode{id} => Term::BlankNode(BlankNode::new_unchecked(id.to_string())),
            Node::LiteralNode{literal, data_type: _, language: _} => Term::Literal(Literal::new_simple_literal(literal)),
        };
        store.insert(Quad::new(s, p, o, GraphName::DefaultGraph));
    }

    let qfmt = "PREFIX brick: <https://brickschema.org/schema/1.1/Brick#>
    PREFIX tag: <https://brickschema.org/schema/1.1/BrickTag#>
    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
    PREFIX owl: <http://www.w3.org/2002/07/owl#>
    PREFIX qudt: <http://qudt.org/schema/qudt/>
    ";
    let q = format!("{}{}", qfmt, "SELECT ?s WHERE { {?s rdf:type brick:Equipment} UNION {?s rdf:type brick:Point} UNION {?s rdf:type brick:Location} }");
    if let QueryResults::Solutions(solutions) =  store.query(&q, QueryOptions::default()).unwrap() {
        for soln in solutions {
            let soln = soln.unwrap();
            println!("{:?} {:?}", soln.get("s"), soln.get("e"));
        }
    };

    let q = format!("{}{}", qfmt, "SELECT ?s ?e WHERE { ?s brick:feeds+ ?e }");
    if let QueryResults::Solutions(solutions) =  store.query(&q, QueryOptions::default()).unwrap() {
        for soln in solutions {
            let soln = soln.unwrap();
            println!("{:?} {:?}", soln.get("s"), soln.get("e"));
        }
    };

    //r.dump_file("output.ttl").unwrap();
}
