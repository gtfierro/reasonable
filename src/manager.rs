use crate::error::Result;
use crate::reasoner::Reasoner;
use crate::common::make_triple;
use log::{debug, info};
use oxigraph::store::sled::SledConflictableTransactionError;
use oxigraph::io::DatasetFormat;
use oxigraph::{
    io::{GraphFormat, GraphParser},
    model::*,
    SledStore,
};
use std::convert::Infallible;
use std::fs;
use std::collections::HashMap;
use std::io::Cursor;
use std::string::String;
use std::time::Instant;

#[allow(non_upper_case_globals)]
const qfmt: &str = "PREFIX brick: <https://brickschema.org/schema/Brick#>
    PREFIX tag: <https://brickschema.org/schema/BrickTag#>
    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
    PREFIX owl: <http://www.w3.org/2002/07/owl#>
    PREFIX qudt: <http://qudt.org/schema/qudt/>
    ";

pub struct TripleUpdate {
    pub updates: HashMap<Triple, i64>
}

impl TripleUpdate {
    pub fn new() -> Self {
        TripleUpdate { updates: HashMap::new() }
    }
    pub fn add_triple(&mut self, triple: Triple) {

        let counter = self.updates.entry(triple).or_insert(0);
        *counter += 1;
    }
    pub fn remove_triple(&mut self, triple: Triple) {
        let counter = self.updates.entry(triple).or_insert(0);
        *counter -= 1;
    }
    pub fn merge(&mut self, other: Self) {
        for (triple, count) in other.updates.into_iter() {
            let counter = self.updates.entry(triple).or_insert(0);
            *counter += count; // handles both the adds and removes
        }
    }
}

pub struct Manager {
    reasoner: Reasoner,
    triple_store: SledStore,
}

impl Manager {
    pub fn new() -> Self {
        Manager {
            reasoner: Reasoner::new(),
            triple_store: SledStore::open("graph.db").unwrap(),
        }
    }

    pub fn new_in_memory() -> Self {
        Manager {
            reasoner: Reasoner::new(),
            triple_store: SledStore::new().unwrap(),
        }
    }

    pub fn size(&self) -> usize {
        self.triple_store.len()
    }

    pub fn store(&self) -> SledStore {
        self.triple_store.clone()
    }

    pub fn dump_string(&self) -> String {
        let mut buffer = Vec::new();
        self.triple_store.dump_dataset(&mut buffer, DatasetFormat::NQuads).unwrap();
        String::from_utf8(buffer).unwrap()
    }

    pub fn load_triples(&mut self, triples: Vec<(String, String, String)>) -> Result<()> {
        let load_triples: Vec<Triple> = triples
            .into_iter()
            .filter_map(|(s_, p_, o_)| {
                let s: Term = {
                    if let Ok(named) = NamedNode::new(&s_) {
                        Term::NamedNode(named)
                    } else if let Ok(bnode) = BlankNode::new(&s_) {
                        Term::BlankNode(bnode)
                    } else {
                        return None;
                    }
                };

                let p: Term = if let Ok(named) = NamedNode::new(p_) {
                    Term::NamedNode(named)
                } else {
                    return None;
                };

                let o: Term = {
                    if let Ok(named) = NamedNode::new(&o_) {
                        Term::NamedNode(named)
                    } else if let Ok(bnode) = BlankNode::new(&o_) {
                        Term::BlankNode(bnode)
                    } else {
                        Term::Literal(Literal::new_simple_literal(&o_))
                    }
                };

                if let Ok(t) = make_triple(s,p,o) {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();
        self.reasoner.load_triples(load_triples);
        self.refresh();
        Ok(())
    }

    pub fn load_file(&mut self, filename: &str) -> Result<()> {
        self.reasoner.load_triples(parse_file(filename)?);
        self.refresh();
        Ok(())
    }

    fn refresh(&mut self) {
        let refresh_start = Instant::now();
        // update the reasoner
        self.reasoner.reason();

        // add reasoned triples to an in-memory store
        self.triple_store
            .transaction(|txn| {
                for t in self.reasoner.view_output().iter() {
                    txn.insert(t.clone().in_graph(GraphNameRef::DefaultGraph).as_ref())?;
                }
                Ok(()) as std::result::Result<(), SledConflictableTransactionError<Infallible>>
            })
            .unwrap();
        info!("now have {} triples", self.triple_store.len());
        info!(
            "refresh completed in {:.02}sec",
            refresh_start.elapsed().as_secs_f64()
        );
    }

    /// Adds the provided triples to the reasoner and re-executes the reasoner
    pub fn add_triples(&mut self, triples: Vec<Triple>) {
        // add new triples to reasoner
        self.reasoner.load_triples(triples);
        self.refresh();
    }

    pub fn process_updates(&mut self, updates: TripleUpdate) {
        // are there deletions? if so, clear out the reasoned state:
        let has_deletions = updates.updates.iter().any(|(_, count)| *count < 0);
        if has_deletions {
            self.reasoner.clear();
        }

        // copy triples out
        let mut triples = self.reasoner.get_input();
        println!("deletions? {} tripels {}", has_deletions, triples.len());
        // remove triples with negative counts
        for (triple, count) in updates.updates.iter() {
            if *count <= 0 {
                println!("removing triple ({})", triples.len());
                triples.retain(|v| *v != *triple);
                println!("after ({})", triples.len());
            } else {
                triples.push(triple.clone());
            }
        }
        for t in &triples {
            println!("adding {:?}", t);
        }

        if has_deletions {
            self.reasoner = Reasoner::new();
            self.triple_store.clear().unwrap();
        }

        self.reasoner.load_triples(triples);
        self.refresh();
    }
}


pub fn parse_file(filename: &str) -> std::result::Result<Vec<Triple>, std::io::Error> {
    let gfmt: GraphFormat = if filename.ends_with(".ttl") {
        GraphFormat::Turtle
    } else if filename.ends_with(".n3") || filename.ends_with(".ntriples") {
        GraphFormat::NTriples
    } else {
        GraphFormat::RdfXml
    };
    let data = fs::read_to_string(filename)?;
    debug!("format: {:?} for {}", gfmt, filename);
    let parser = GraphParser::from_format(gfmt);
    let triples: std::result::Result<Vec<Triple>, std::io::Error> =
        parser.read_triples(Cursor::new(data))?
        .collect(); //.collect::<Result<Triple>>();
    triples
}
