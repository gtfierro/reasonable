use crate::owl::Reasoner;
use crate::error::{ReasonableError, Result};
use std::fmt;
use std::fs;
use std::io::Cursor;
use std::error::Error;
use rdf::{
    node::Node,
    uri::Uri,
};
use std::collections::HashMap;
use oxigraph::{
    MemoryStore,
    sparql::{
        Query,
        QueryOptions,
        QueryResults
    },
    model::*,
    io::{
        GraphFormat,
        GraphParser,
    }

};

macro_rules! uri {
    ($t:expr) => (Node::UriNode{uri: Uri::new($t)});
}
macro_rules! bnode {
    ($t:expr) => (Node::BlankNode{id: $t});
}
macro_rules! literal {
    ($t:expr, $d:expr, $l:expr) => (Node::LiteralNode{literal: $t, data_type: $d, language: $l});
}

const qfmt: &'static str = "PREFIX brick: <https://brickschema.org/schema/1.1/Brick#>
    PREFIX tag: <https://brickschema.org/schema/1.1/BrickTag#>
    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
    PREFIX owl: <http://www.w3.org/2002/07/owl#>
    PREFIX qudt: <http://qudt.org/schema/qudt/>
    ";

pub struct ViewRef<'a> {
    // columns: &'a Vec<String>,
    md: &'a ViewMetadata,
}

impl<'a> ViewRef<'a> {
    pub fn columns(&self) -> &'a[String] {
        &self.md.columns
    }
}

pub struct ViewMetadata {
    query: Query,
    table_name: String,
    columns: Vec<String>,
}

impl fmt::Display for ViewMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<name: {}, cols {:?}>", self.table_name, self.columns)
    }
}

pub struct Manager {
    reasoner: Reasoner,
    triple_store: MemoryStore,
    views: HashMap<String, ViewMetadata>,
}

impl Manager {
    pub fn new() -> Self {
        Manager {
            reasoner: Reasoner::new(),
            triple_store: MemoryStore::new(),
            views: HashMap::new(),
        }
    }

    pub fn load_file(&mut self, filename: &str) -> Result<()> {
        let gfmt: GraphFormat = if filename.ends_with(".ttl") {
            GraphFormat::Turtle
        } else if filename.ends_with(".n3") || filename.ends_with(".ntriples") {
            GraphFormat::NTriples
        } else {
            GraphFormat::RdfXml
        };
        let data = fs::read_to_string(filename)?;
        println!("format: {:?} for {}", gfmt, filename);
        let parser = GraphParser::from_format(gfmt);
        let triples: Vec<std::result::Result<Triple, std::io::Error>> = parser.read_triples(Cursor::new(data))?.collect();//.collect::<Result<Triple>>();
        let load_triples: Vec<(Node, Node, Node)> = triples.into_iter().filter_map(|tres| {
            match tres {
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
                        Term::Literal(lit) => literal!(lit.value().to_string(),
                                                       Some(Uri::new(lit.datatype().to_string())),
                                                       match lit.language() {
                                                           Some(l) => Some(l.to_string()),
                                                           None => None
                                                       })
                    };
                    Some((s, p, o))
                }
            }
        }).collect();
        self.reasoner.load_triples(load_triples);
        self.refresh();
        Ok(())
    }

    fn refresh(&mut self) {
        // update the reasoner
        self.reasoner.reason();

        // add reasoned triples to an in-memory store
        for t in self.reasoner.get_triples() {
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
            self.triple_store.insert(Quad::new(s, p, o, GraphName::DefaultGraph));
        }
    }

    /// Adds the provided triples to the reasoner and re-executes the reasoner
    pub fn add_triples(&mut self, triples: Vec<(Node, Node, Node)>) {
        // add new triples to reasoner
        self.reasoner.load_triples(triples);
        self.refresh();
    }

    /// Adds a view with the given name, defined by the provided SPARQL query
    pub fn add_view(&mut self, name: String, query: &str) -> Result<ViewRef> {
        // execute query to get the schema?
        let sparql = format!("{}{}", qfmt, query);

        let q = Query::parse(&sparql, None)?;

        println!("query: {}", sparql);
        let res = self.triple_store.query(&sparql, QueryOptions::default())?;
        if let QueryResults::Solutions(solutions) = res {
            let name_key = name.clone();
            let view_key = name.clone();
            let md = ViewMetadata{
                query: q,
                table_name: name,
                columns: solutions.variables()
                                  .to_vec()
                                  .into_iter()
                                  .map(|t| t.into_string())
                                  .collect(),
            };
            self.views.insert(name_key, md);

            return Ok(ViewRef{
                md: self.views.get(&view_key).unwrap(),
            })

            //for soln in solutions {
            //    let soln = soln.unwrap();
            //    println!("{:?} {:?}", soln.get("x"), soln.get("y"));
            //}
        };
        Err(ReasonableError::ManagerError("no solutions".to_string()))
    }
}
