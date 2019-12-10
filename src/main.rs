extern crate rdf;
#[macro_use] extern crate itertools;

mod index;
mod types;
mod owl;
mod floyd_warshall;
use crate::owl::Reasoner;

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

    let mut r = Reasoner::new();

    // TODO: load in datasets
    r.load_file("rdfs.ttl").unwrap();
    // Brick.ttl has some parse error so we use n3
    r.load_file("Brick.n3").unwrap();
    r.load_file("example.n3").unwrap();

    let v1 : Vec::<(&str, &str, &str)> = vec![
        ("a", RDF_TYPE, "Class1"),
        ("b", RDF_TYPE, "Class1"),
        ("Class1", RDFS_SUBCLASSOF, "Class2"),
        ("Class3", RDFS_SUBCLASSOF, "Class2"),
        ("https://brickschema.org/schema/1.1.0/Brick#feeds", RDFS_DOMAIN, "Class2"),
        ("https://brickschema.org/schema/1.1.0/Brick#feeds", RDFS_DOMAIN, "Class3"),
        ("https://brickschema.org/schema/1.1.0/Brick#isFedBy",OWL_INVERSEOF, "https://brickschema.org/schema/1.1.0/Brick#feeds"),
        ("c", "https://brickschema.org/schema/1.1.0/Brick#feeds", "d"),

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
