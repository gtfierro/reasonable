use crate::error::{ReasonableError, Result};
use crate::reasoner::Reasoner;
use log::{debug, info};
use oxigraph::{
    io::{GraphFormat, GraphParser},
    model::*,
    sparql::{QueryOptions, QueryResults},
    store::memory::MemoryPreparedQuery,
    MemoryStore,
};
use rdf::{node::Node, uri::Uri};
use std::collections::HashMap;
use std::fmt;
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

pub struct ViewRef<'a> {
    // columns: &'a Vec<String>,
    md: &'a ViewMetadata,
}

impl<'a> ViewRef<'a> {
    pub fn columns(&self) -> &'a [String] {
        &self.md.columns
    }
    pub fn name(&self) -> &str {
        &self.md.table_name
    }

    pub fn contents(&self) -> Result<Vec<Vec<Term>>> {
        let res = self.md.query.exec()?;
        let mut rows: Vec<Vec<Term>> = Vec::new();
        if let QueryResults::Solutions(solutions) = res {
            for soln in solutions {
                let vals = soln?;
                let mut row: Vec<Term> = Vec::new();
                for col in self.md.columns.iter() {
                    row.push(vals.get(col.as_str()).unwrap().clone());
                }
                rows.push(row);
            }
        }
        Ok(rows)
    }
    pub fn contents_string(&self) -> Result<Vec<Vec<String>>> {
        let res = self.md.query.exec()?;
        let mut rows: Vec<Vec<String>> = Vec::new();
        if let QueryResults::Solutions(solutions) = res {
            for soln in solutions {
                let vals = soln?;
                let mut row: Vec<String> = Vec::new();
                for col in self.md.columns.iter() {
                    row.push(vals.get(col.as_str()).unwrap().to_string());
                }
                rows.push(row);
            }
        }
        Ok(rows)
    }
}

pub struct ViewMetadata {
    pub query_string: String,
    pub table_name: String,
    query: MemoryPreparedQuery,
    columns: Vec<String>,
}

impl ViewMetadata {
    pub fn contents_string(&self) -> Result<Vec<Vec<String>>> {
        let res = self.query.exec()?;
        let mut rows: Vec<Vec<String>> = Vec::new();
        if let QueryResults::Solutions(solutions) = res {
            for soln in solutions {
                let vals = soln?;
                let mut row: Vec<String> = Vec::new();
                for col in self.columns.iter() {
                    let s = match vals.get(col.as_str()).unwrap() {
                        Term::NamedNode(named) => named.clone().into_string(),
                        Term::BlankNode(bnode) => bnode.clone().into_string(),
                        Term::Literal(lit) => lit.value().to_string(),
                    };
                    row.push(s);
                }
                rows.push(row);
            }
        }
        Ok(rows)
    }
    pub fn name(&self) -> &str {
        &self.table_name
    }
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    pub fn get_insert_sql(&self) -> String {
        let cols: String = self
            .columns()
            .to_vec()
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        let inps: String = (0..self.columns().len())
            .map(|_| "?".to_string())
            .collect::<Vec<String>>()
            .join(", ");
        format!(
            "INSERT INTO view_{}({}) VALUES ({});",
            self.table_name, cols, inps
        )
    }

    pub fn get_create_tab(&self) -> String {
        let cols: String = self
            .columns()
            .to_vec()
            .iter()
            .map(|c| format!("{} TEXT", c))
            .collect::<Vec<String>>()
            .join(", ");
        format!(
            "CREATE TABLE IF NOT EXISTS view_{}({});",
            self.table_name, cols
        )
    }

    pub fn get_delete_tab(&self) -> String {
        format!("DELETE FROM view_{};", self.table_name)
    }
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

    pub fn size(&self) -> usize {
        self.triple_store.len()
    }

    pub fn store(&self) -> MemoryStore {
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
        for t in self.reasoner.view_output().iter() {
            let s = match &t.0 {
                Node::UriNode { uri } => {
                    NamedOrBlankNode::NamedNode(NamedNode::new_unchecked(uri.to_string()))
                }
                Node::BlankNode { id } => {
                    NamedOrBlankNode::BlankNode(BlankNode::new_unchecked(id.to_string()))
                }
                _ => panic!("no subject literals"),
            };
            let p = match &t.1 {
                Node::UriNode { uri } => NamedNode::new_unchecked(uri.to_string()),
                _ => panic!("no must be named node"),
            };
            let o = match &t.2 {
                Node::UriNode { uri } => Term::NamedNode(NamedNode::new_unchecked(uri.to_string())),
                Node::BlankNode { id } => Term::BlankNode(BlankNode::new_unchecked(id.to_string())),
                Node::LiteralNode {
                    literal,
                    data_type: _,
                    language: _,
                } => Term::Literal(Literal::new_simple_literal(literal)),
            };
            self.triple_store
                .insert(Quad::new(s, p, o, GraphName::DefaultGraph));
        }
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

    /// Adds a view with the given name, defined by the provided SPARQL query
    pub fn add_view(&mut self, name: String, query: &str) -> Result<ViewRef> {
        // execute query to get the schema?
        let sparql = format!("{}{}", qfmt, query);

        let q = self
            .triple_store
            .prepare_query(&sparql, QueryOptions::default())?;

        debug!("query: {}", sparql);
        let res = q.exec()?;
        if let QueryResults::Solutions(solutions) = res {
            let name_key = name.clone();
            let view_key = name.clone();
            let md = ViewMetadata {
                query: q,
                query_string: query.to_string(),
                table_name: name,
                columns: solutions
                    .variables()
                    .to_vec()
                    .into_iter()
                    .map(|t| t.into_string())
                    .collect(),
            };
            self.views.insert(name_key, md);

            return Ok(ViewRef {
                md: self.views.get(&view_key).unwrap(),
            });
        };
        Err(ReasonableError::ManagerError("no solutions".to_string()))
    }

    pub fn add_view2(&self, name: String, query: &str) -> Result<ViewMetadata> {
        let sparql = format!("{}{}", qfmt, query);

        let q = self
            .triple_store
            .prepare_query(&sparql, QueryOptions::default())?;

        debug!("query: {}", sparql);
        let res = q.exec()?;
        if let QueryResults::Solutions(solutions) = res {
            return Ok(ViewMetadata {
                query: q,
                query_string: query.to_string(),
                table_name: name,
                columns: solutions
                    .variables()
                    .to_vec()
                    .into_iter()
                    .map(|t| t.into_string())
                    .collect(),
            });
        }
        Err(ReasonableError::ManagerError("no solutions".to_string()))
    }

    pub fn get_view(&self, view: &ViewRef) -> Result<Vec<Vec<Term>>> {
        let res = view.md.query.exec()?;
        let mut rows: Vec<Vec<Term>> = Vec::new();
        if let QueryResults::Solutions(solutions) = res {
            for soln in solutions {
                let vals = soln?;
                let mut row: Vec<Term> = Vec::new();
                for col in view.md.columns.iter() {
                    row.push(vals.get(col.as_str()).unwrap().clone());
                }
                rows.push(row);
            }
        }
        Ok(rows)
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
