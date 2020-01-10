extern crate rdf;
extern crate itertools;
extern crate log;

mod index;
mod owl;
mod disjoint_sets;
#[allow(dead_code)]
mod common;
use crate::owl::Reasoner;
use std::time::Instant;
use std::env;
use log::info;

#[allow(dead_code)]
const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
#[allow(dead_code)]
const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
#[allow(dead_code)]
const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
#[allow(dead_code)]
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
#[allow(dead_code)]
const OWL_INVERSEOF: &str = "http://www.w3.org/2002/07/owl#inverseOf";

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
    r.dump_file("output.ttl").unwrap();
    // r.dump_file("output.n3").unwrap();

    // // TODO: load in datasets
    // r.load_file("rdfs.ttl").unwrap();
    // // Brick.ttl has some parse error so we use n3
    // r.load_file("Brick.n3").unwrap();
    // if args.len() > 1 {
    //     r.load_file(args.get(1).unwrap()).unwrap();
    // } else {
    //     r.load_file("example.n3").unwrap();
    // }

    // let v1 : Vec::<(&str, &str, &str)> = vec![
    //     ("a", RDF_TYPE, "Class1"),
    //     ("b", RDF_TYPE, "Class1"),
    //     ("Class1", RDFS_SUBCLASSOF, "Class2"),
    //     ("Class3", RDFS_SUBCLASSOF, "Class2"),
    //     ("https://brickschema.org/schema/1.1.0/Brick#feeds", RDFS_DOMAIN, "Class2"),
    //     ("https://brickschema.org/schema/1.1.0/Brick#feeds", RDFS_DOMAIN, "Class3"),
    //     ("https://brickschema.org/schema/1.1.0/Brick#isFedBy",OWL_INVERSEOF, "https://brickschema.org/schema/1.1.0/Brick#feeds"),
    //     ("c", "https://brickschema.org/schema/1.1.0/Brick#feeds", "d"),

    //     // cls-thing
    //     ("owl:Thing", "rdf:type", "owl:Class"),
    //     // cls-nothing1
    //     ("owl:Nothing", "rdf:type", "owl:Class"),

    //     // owl definitions
    //     ("owl:inverseOf", "rdf:type", "owl:SymmetricProperty"),
    // ];
    // r.load_triples(v1);

}
