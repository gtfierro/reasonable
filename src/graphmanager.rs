use crate::reasoner::Reasoner;
use oxigraph::{
    model::*,
    //store::memory::MemoryPreparedQuery,
    //MemoryStore
    SledStore,
};
use rdf::node::Node;
use oxigraph::store::sled::SledConflictableTransactionError;
use std::convert::Infallible;
use std::collections::HashMap;
use std::string::String;
use std::time::Instant;

pub struct GraphManager {
    triple_store: SledStore,
    reasoners: HashMap<String, Reasoner>,
}

impl GraphManager {
    pub fn new() -> Self {
        GraphManager {
            reasoners: HashMap::new(),
            triple_store: SledStore::open("graph.db").unwrap(),
        }
    }

    pub fn add_triples(&mut self,  graph: Option<String>, triples: Vec<(Node, Node, Node)>) {
        let graphname = match graph {
            Some(g) => g,
            None => "default".to_owned(),
        };
        self.reasoners.entry(graphname)
                      .or_insert(Reasoner::new())
                      .load_triples(triples);
        self.refresh();
    }

    fn refresh(&mut self) {
        let refresh_start = Instant::now();
        // update the reasoner
        for (graphname, reasoner) in self.reasoners.iter_mut() {
            reasoner.reason();

            let graphurn = format!("urn:{}", graphname);
            let graph = GraphNameRef::NamedNode(NamedNodeRef::new(&graphurn).unwrap());
            println!("Inserting triples into {}", graph);


            // add reasoned triples to an in-memory store
            self.triple_store.transaction(|txn| {
                for t in reasoner.view_output().iter() {
                    let s = match &t.0 {
                        Node::UriNode { uri } => {
                            NamedOrBlankNodeRef::NamedNode(NamedNodeRef::new_unchecked(uri.to_string()))
                        }
                        Node::BlankNode { id } => {
                            NamedOrBlankNodeRef::BlankNode(BlankNodeRef::new_unchecked(id))
                        }
                        Node::LiteralNode { literal, data_type: _, language: _} => {
                            println!("No subject literals! {}", literal);
                            continue;
                        }
                    };
                    let p = match &t.1 {
                        Node::UriNode { uri } => NamedNodeRef::new_unchecked(uri.to_string()),
                        _ => panic!("no must be named node"),
                    };
                    let o = match &t.2 {
                        Node::UriNode { uri } => TermRef::NamedNode(NamedNodeRef::new_unchecked(uri.to_string())),
                        Node::BlankNode { id } => TermRef::BlankNode(BlankNodeRef::new_unchecked(id)),
                        Node::LiteralNode {
                            literal,
                            data_type: _,
                            language: _,
                        } => TermRef::Literal(LiteralRef::new_simple_literal(literal)),
                    };
                    txn.insert(QuadRef::new(s, p, o, graph))?;
                }
                Ok(()) as std::result::Result<(),SledConflictableTransactionError<Infallible>>
            }).unwrap();
        }
        println!("now have {} triples", self.triple_store.len());
        println!(
            "refresh completed in {:.02}sec",
            refresh_start.elapsed().as_secs_f64()
        );
    }

    pub fn store(&self) -> SledStore {
        self.triple_store.clone()
    }
}
