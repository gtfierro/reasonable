use crate::reasoner::Reasoner;
use oxigraph::store::sled::SledConflictableTransactionError;
use oxigraph::{
    model::*,
    //store::memory::MemoryPreparedQuery,
    //MemoryStore
    SledStore,
};
use std::collections::HashMap;
use std::convert::Infallible;
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

    pub fn add_triples(&mut self, graph: Option<String>, triples: Vec<Triple>) {
        let graphname = match graph {
            Some(g) => g,
            None => "default".to_owned(),
        };
        self.reasoners
            .entry(graphname.clone())
            .or_insert(Reasoner::new())
            .load_triples(triples);
        self.refresh_graph(&graphname);
    }

    fn refresh_graph(&mut self, graphname: &str) {
        println!("Refreshing {}", graphname);
        let reasoner = self.reasoners.get_mut(graphname).unwrap();
        reasoner.reason();
        let graphurn = format!("urn:{}", graphname);
        let graph = GraphNameRef::NamedNode(NamedNodeRef::new(&graphurn).unwrap());

        // add reasoned triples to an in-memory store
        self.triple_store
            .transaction(|txn| {
                for t in reasoner.view_output().iter() {
                    txn.insert(t.clone().in_graph(graph).as_ref())?;
                }
                Ok(()) as std::result::Result<(), SledConflictableTransactionError<Infallible>>
            })
            .unwrap();
    }

    fn refresh(&mut self) {
        let refresh_start = Instant::now();
        // update the reasoner
        for (graphname, reasoner) in self.reasoners.iter_mut() {
            reasoner.reason();

            let graphurn = format!("urn:{}", graphname);
            let graph = GraphNameRef::NamedNode(NamedNodeRef::new(&graphurn).unwrap());

            // add reasoned triples to an in-memory store
            self.triple_store
                .transaction(|txn| {
                    for t in reasoner.view_output().iter() {
                        txn.insert(t.clone().in_graph(graph).as_ref())?;
                    }
                    Ok(()) as std::result::Result<(), SledConflictableTransactionError<Infallible>>
                })
                .unwrap();
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
