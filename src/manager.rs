use crate::error::Result;
use crate::reasoner::Reasoner;
use log::{debug, info};
use oxigraph::store::sled::SledConflictableTransactionError;
use oxigraph::{
    io::{GraphFormat, GraphParser},
    model::*,
    SledStore,
};
use rdf::{node::Node, uri::Uri};
use std::convert::Infallible;
use std::fs;
use std::io::Cursor;
use std::string::String;
use std::time::Instant;

macro_rules! uri {
    ($t:expr) => {
        Node::UriNode { uri: Uri::new($t) }
    };
}
macro_rules! bnode {
    ($t:expr) => {
        Node::BlankNode { id: $t }
    };
}
macro_rules! literal {
    ($t:expr, $d:expr, $l:expr) => {
        Node::LiteralNode {
            literal: $t,
            data_type: $d,
            language: $l,
        }
    };
}

#[allow(non_upper_case_globals)]
const qfmt: &str = "PREFIX brick: <https://brickschema.org/schema/1.1/Brick#>
    PREFIX tag: <https://brickschema.org/schema/1.1/BrickTag#>
    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
    PREFIX owl: <http://www.w3.org/2002/07/owl#>
    PREFIX qudt: <http://qudt.org/schema/qudt/>
    ";

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

    pub fn size(&self) -> usize {
        self.triple_store.len()
    }

    pub fn store(&self) -> SledStore {
        self.triple_store.clone()
    }

    pub fn load_triples(&mut self, triples: Vec<(String, String, String)>) -> Result<()> {
        let load_triples: Vec<(Node, Node, Node)> = triples
            .into_iter()
            .filter_map(|(s_, p_, o_)| {
                let s: Node = {
                    if let Ok(named) = NamedNode::new(s_.clone()) {
                        uri!(named.into_string())
                    } else if let Ok(bnode) = BlankNode::new(s_) {
                        bnode!(bnode.into_string())
                    } else {
                        return None;
                    }
                };

                let p: Node = uri!(p_);

                let o: Node = {
                    if let Ok(named) = NamedNode::new(o_.clone()) {
                        uri!(named.into_string())
                    } else if let Ok(bnode) = BlankNode::new(o_.clone()) {
                        bnode!(bnode.into_string())
                    } else {
                        literal!(o_, None, None)
                    }
                };

                Some((s, p, o))
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
                    let s = match &t.0 {
                        Node::UriNode { uri } => NamedOrBlankNodeRef::NamedNode(
                            NamedNodeRef::new_unchecked(uri.to_string()),
                        ),
                        Node::BlankNode { id } => {
                            NamedOrBlankNodeRef::BlankNode(BlankNodeRef::new_unchecked(id))
                        }
                        _ => panic!("no subject literals"),
                    };
                    let p = match &t.1 {
                        Node::UriNode { uri } => NamedNodeRef::new_unchecked(uri.to_string()),
                        _ => panic!("no must be named node"),
                    };
                    let o = match &t.2 {
                        Node::UriNode { uri } => {
                            TermRef::NamedNode(NamedNodeRef::new_unchecked(uri.to_string()))
                        }
                        Node::BlankNode { id } => {
                            TermRef::BlankNode(BlankNodeRef::new_unchecked(id))
                        }
                        Node::LiteralNode {
                            literal,
                            data_type: _,
                            language: _,
                        } => TermRef::Literal(LiteralRef::new_simple_literal(literal)),
                    };
                    txn.insert(QuadRef::new(s, p, o, GraphNameRef::DefaultGraph))?;
                    //self.triple_store
                    //    .insert(QuadRef::new(s, p, o, GraphNameRef::DefaultGraph)).unwrap();
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
    pub fn add_triples(&mut self, triples: Vec<(Node, Node, Node)>) {
        // add new triples to reasoner
        self.reasoner.load_triples(triples);
        self.refresh();
    }
}


pub fn parse_file(filename: &str) -> Result<Vec<(Node, Node, Node)>> {
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
    let triples: Vec<std::result::Result<Triple, std::io::Error>> =
        parser.read_triples(Cursor::new(data))?.collect(); //.collect::<Result<Triple>>();
    Ok(triples
        .into_iter()
        .filter_map(|tres| match tres {
            Err(_) => None,
            Ok(t) => {
                let s = match t.subject {
                    NamedOrBlankNode::NamedNode(node) => uri!(node.into_string()),
                    NamedOrBlankNode::BlankNode(id) => bnode!(id.into_string()),
                };
                let p = uri!(t.predicate.into_string());
                let o = match t.object {
                    Term::NamedNode(node) => uri!(node.into_string()),
                    Term::BlankNode(id) => bnode!(id.into_string()),
                    Term::Literal(lit) => literal!(
                        lit.value().to_string(),
                        Some(Uri::new(lit.datatype().to_string())),
                        match lit.language() {
                            Some(l) => Some(l.to_string()),
                            None => None,
                        }
                    ),
                };
                Some((s, p, o))
            }
        })
        .collect())
}
