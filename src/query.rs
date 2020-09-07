use crate::owl::node_to_string;
use rdf::graph;
use rdf::node::Node;
use rdf::triple::Triple;
use std::ops::Fn;
use std::rc::Rc;
use std::fmt;

//use ketos::{Error, Interpreter, Value};

enum JoinHeader {
    Left(usize),
    Right(usize),
}

pub struct Graph {
    graph: graph::Graph,
}

impl From<Vec<(Node, Node, Node)>> for Graph {
    fn from(triples: Vec<(Node, Node, Node)>) -> Self {
        let mut graph = graph::Graph::new(None);
        let triples: Vec<Triple> = triples.iter().map(|t| {
            Triple::new(&t.0, &t.1, &t.2)
        }).collect();
        graph.add_triples(&triples);
        Graph{graph: graph}
    }
}

pub struct Relation<'a> {
    header: Vec<String>,
    rows: Vec<Vec<&'a Node>>,
}

impl<'a> Relation<'a> {
    fn from(triples: Vec<&'a Triple>, header: Vec<String>) -> Self {
        let rows: Vec<Vec<&'a Node>> = triples.iter().map(|t| {
            vec![t.subject(), t.predicate(), t.object()]
        }).collect();
        Relation{
            header: header,
            rows: rows,
        }
    }

    //fn joinf(first: &Relation, other: &Relation, dojoin: &dyn Fn(&Vec<&Node>, &Vec<&Node>) -> bool) -> Self {
    //        let new: Vec<Vec<Node>> = Vec::new();
    //        for t1 in first.rows.iter() {
    //            for t2 in other.rows.iter() {
    //                if dojoin(t1, t2) {

    //                    println!("JOIN {:?} {:?}", t1, t2);
    //                }
    //            }
    //        }
    //        Relation {header: None, rows: Vec::new() }
    //}

    fn join(first: Relation<'a>, other: Relation<'a>) -> Self {
            let mut new: Vec<Vec<&Node>> = Vec::new();
            let mut indices: Vec<(usize, usize)> = Vec::new();
            let mut new_indices: Vec<JoinHeader> = Vec::new();
            let mut new_header: Vec<String> = Vec::new();

            let t1h = &first.header;
            let t2h = &other.header;
            for (i1, v1) in t1h.iter().enumerate() {
                if v1 == "_" {
                    continue
                } else if !new_header.contains(v1) {
                    new_indices.push(JoinHeader::Left(i1));
                    new_header.push(v1.to_string());
                }

                for (i2, v2) in t2h.iter().enumerate() {
                    if v2 == "_" {
                        continue
                    }
                    if v1 == v2 {
                        indices.push((i1, i2));
                    } else if !new_header.contains(v2) {
                        println!("add {} to {:?}", v2, new_header);
                        new_indices.push(JoinHeader::Right(i2));
                        new_header.push(v2.to_string());
                    }
                }
            }

            for t1 in first.rows.iter() {
                for t2 in other.rows.iter() {
                    if !indices.iter().all(|(i1, i2)| t1[*i1] == t2[*i2]) {
                        continue
                    }
                    let mut row: Vec<&Node> = Vec::new();
                    for jh in &new_indices {
                        match jh {
                            JoinHeader::Left(idx) => row.push(t1.get(*idx).unwrap()),
                            JoinHeader::Right(idx) => row.push(t2.get(*idx).unwrap()),
                        }
                    }
                    new.push(row);
                }
            }
            Relation {header: new_header, rows: new }
    }
}

impl<'a> fmt::Display for Relation<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}\n", self.header)?;
        for t in self.rows.iter() {
            write!(f, "[")?;
            for node in t.iter() {
                write!(f, "{} ", node_to_string(node))?;
            }
            write!(f, "]\n")?;
        }
        Ok(())
    }
}

pub struct Context<'a> {
    graph: &'a Graph,
    relations: Vec<Relation<'a>>,
}

impl<'a> Context<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Context{graph: graph, relations: Vec::new()}
    }

    pub fn query(&self, query: String) {
        //let interp = Interpreter::new();
        //interp.run_code(&query, None).unwrap();
    }

    pub fn with_subject(&mut self, s: &Node, p: String, o: String){
        let triples = Relation::from(self.graph.graph.get_triples_with_subject(s), vec!["_".to_string(), p, o]);
        self.relations.push(triples);
    }

    pub fn with_object(&mut self, s:String, p: String, o: &Node){
        let triples = Relation::from(self.graph.graph.get_triples_with_object(o), vec![s, p, "_".to_string()]);
        self.relations.push(triples);
    }

    pub fn with_predicate(&mut self, s:String, p: &Node, o: String){
        let triples = Relation::from(self.graph.graph.get_triples_with_predicate(p), vec![s, "_".to_string(), o]);
        self.relations.push(triples);
    }

    pub fn with_subject_object(&mut self, s: &Node, p: String, o: &Node){
        let triples = Relation::from(self.graph.graph.get_triples_with_subject_and_object(s, o), vec!["_".to_string(), p, "_".to_string()]);
        self.relations.push(triples);
    }

    pub fn with_subject_predicate(&mut self, s: &Node, p: &Node, o: String){
        let triples = Relation::from(self.graph.graph.get_triples_with_subject_and_predicate(s, p), vec!["_".to_string(), "_".to_string(), o]);
        self.relations.push(triples);
    }

    pub fn with_predicate_object(&mut self, s: String, p: &Node, o: &Node){
        let triples = Relation::from(self.graph.graph.get_triples_with_predicate_and_object(p, o), vec![s, "_".to_string(), "_".to_string()]);
        self.relations.push(triples);
    }

    pub fn joinall(mut self) -> Relation<'a> {
        let r1 = self.relations.remove(0);
        let r2 = self.relations.remove(0);
        let acc = Relation::join(r1, r2);
        self.relations.into_iter().skip(2).fold(acc, |a, b| {
            Relation::join(a, b)
        })
    }

    pub fn resolve(mut self) -> Option<Relation<'a>> {
        match self.relations.len() {
            0 => None,
            1 => Some(self.relations.remove(0)),
            _ => Some(self.joinall())
        }
    }
}
