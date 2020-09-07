use ::reasonable::owl::Reasoner;
use ::reasonable::query;
use rdf::node::Node;
use rdf::uri::Uri;
use std::env;
use std::time::Instant;
use log::info;

macro_rules! uri {
    ($ns:expr, $t:expr) => (Node::UriNode{uri: Uri::new(format!($ns, $t))});
}
macro_rules! rdf {
    ($t:expr) => (uri!("http://www.w3.org/1999/02/22-rdf-syntax-ns#{}", $t));
}
macro_rules! rdfs {
    ($t:expr) => (uri!("http://www.w3.org/2000/01/rdf-schema#{}", $t));
}
macro_rules! owl {
    ($t:expr) => (uri!("http://www.w3.org/2002/07/owl#{}", $t));
}
macro_rules! brick {
    ($t:expr) => (uri!("https://brickschema.org/schema/1.1/Brick#{}", $t));
}
macro_rules! ex {
    ($t:expr) => (uri!("http://buildsys.org/ontologies/building_example#{}", $t));
}

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

    let triples = r.get_triples();

    let g = query::Graph::from(triples);
    let mut ctx = query::Context::new(&g);
    // ctx.query(r#"(println "hello")"#.to_string());

    ctx.with_predicate_object("ahu".to_string(), &rdf!("type"), &brick!("AHU"));
    ctx.with_predicate("ahu".to_string(), &rdf!("type"), "type".to_string());
    ctx.with_predicate("ahu".to_string(), &brick!("feeds"), "equip".to_string());
    println!("{}", ctx.resolve().unwrap());

    r.dump_file("output.ttl").unwrap();
}
