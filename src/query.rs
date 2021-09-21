use crate::owl::node_to_string;
use datafrog::Iteration;
use farmhash;
use rdf::graph;
use rdf::node::Node;
use rdf::triple::Triple;
use rdf::uri::Uri;
use regex::Regex;
//use std::ops::Fn;
//use std::rc::Rc;
use serde::{Deserialize, Serialize};
use serde_sexpr::{from_str, to_string, Error};
use std::collections::HashMap;
use std::fmt;

macro_rules! uri {
    ($ns:expr, $t:expr) => {
        Node::UriNode {
            uri: Uri::new(format!("{}{}", $ns, $t)),
        }
    };
}

macro_rules! skip_none {
    ($res:expr) => {
        match $res {
            Some(val) => val,
            None => {
                continue;
            }
        }
    };
}

//use ketos::{Error, Interpreter, Value};
//
// idea: s-expr parse into a struct that has the prefix list, select, where clauses?
#[derive(Serialize, Deserialize, Debug)]
struct Prefix {
    ns: String,
    uri: String,
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@prefix {}={}", self.ns, self.uri)
    }
}

#[derive(Debug)]
enum Atom {
    Var(String),
    Node(Node),
}

#[derive(Serialize, Deserialize, Debug)]
struct Query {
    prefixes: Vec<[String; 2]>,
    select: Vec<String>,
    clauses: Vec<[String; 3]>,
}

impl Query {
    fn to_atom(&self, s: &str) -> Atom {
        if s.starts_with("?") {
            Atom::Var(s.to_string())
        } else {
            for pfx in &self.prefixes {
                let npfx = format!("{}:", pfx[0]);
                if s.starts_with(&npfx) {
                    let v: &str = s.split(":").skip(1).next().unwrap();
                    return Atom::Node(uri!(pfx[1], v));
                }
            }
            return Atom::Node(uri!("", s));
        }
    }
    fn parse(&self) -> Vec<[Atom; 3]> {
        let mut parsed: Vec<[Atom; 3]> = Vec::new();
        for clause in &self.clauses {
            let pattern: [Atom; 3] = [
                self.to_atom(&clause[0]),
                self.to_atom(&clause[1]),
                self.to_atom(&clause[2]),
            ];
            parsed.push(pattern);
        }
        parsed
    }
}

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
        let triples: Vec<Triple> = triples
            .iter()
            .map(|t| Triple::new(&t.0, &t.1, &t.2))
            .collect();

        // TODO: use the Iteration to compute transitive closure for properties

        graph.add_triples(&triples);
        Graph { graph: graph }
    }
}

impl Graph {
    pub fn query(&self, q: String) -> Option<Relation> {
        let re = Regex::new(r"([\t\n\r]+)|(  +)").unwrap();
        let q = re.replace_all(&q, "");
        println!("parse {}", q);
        let query = from_str::<Query>(&q).unwrap();
        let mut ctx = Context::new(self);
        let clauses = query.parse();
        for clause in clauses {
            println!("{:?}", clause);
            match clause {
                [Atom::Var(s), Atom::Node(p), Atom::Node(o)] => {
                    ctx.with_predicate_object(s, &p, &o)
                }
                [Atom::Node(s), Atom::Var(p), Atom::Node(o)] => ctx.with_subject_object(&s, p, &o),
                [Atom::Node(s), Atom::Node(p), Atom::Var(o)] => {
                    ctx.with_subject_predicate(&s, &p, o)
                }
                [Atom::Node(s), Atom::Var(p), Atom::Var(o)] => ctx.with_subject(&s, p, o),
                [Atom::Var(s), Atom::Node(p), Atom::Var(o)] => ctx.with_predicate(s, &p, o),
                [Atom::Var(s), Atom::Var(p), Atom::Node(o)] => ctx.with_object(s, p, &o),
                [Atom::Var(_), Atom::Var(_), Atom::Var(_)] => panic!("not covered"),
                [Atom::Node(_), Atom::Node(_), Atom::Node(_)] => panic!("not covered"),
            };
        }
        match ctx.resolve() {
            Some(result) => Some(result.project(&query.select)),
            None => None,
        }
    }
}

pub struct Relation<'a> {
    header: Vec<String>,
    rows: Vec<Vec<&'a Node>>,
}

impl<'a> Relation<'a> {
    fn from(triples: Vec<&'a Triple>, header: Vec<String>) -> Self {
        let rows: Vec<Vec<&'a Node>> = triples
            .iter()
            .map(|t| vec![t.subject(), t.predicate(), t.object()])
            .collect();
        Relation {
            header: header,
            rows: rows,
        }
    }

    fn join(first: Relation<'a>, other: Relation<'a>) -> Self {
        let mut new: Vec<Vec<&Node>> = Vec::new();
        let mut indices: Vec<(usize, usize)> = Vec::new();
        let mut new_indices: Vec<JoinHeader> = Vec::new();
        let mut new_header: Vec<String> = Vec::new();

        let t1h = &first.header;
        let t2h = &other.header;
        for (i1, v1) in t1h.iter().enumerate() {
            if v1 == "_" {
                continue;
            } else if !new_header.contains(v1) {
                new_indices.push(JoinHeader::Left(i1));
                new_header.push(v1.to_string());
            }

            for (i2, v2) in t2h.iter().enumerate() {
                if v2 == "_" {
                    continue;
                }
                if v1 == v2 {
                    indices.push((i1, i2));
                } else if !new_header.contains(v2) {
                    new_indices.push(JoinHeader::Right(i2));
                    new_header.push(v2.to_string());
                }
            }
        }

        for t1 in first.rows.iter() {
            for t2 in other.rows.iter() {
                if !indices.iter().all(|(i1, i2)| t1[*i1] == t2[*i2]) {
                    continue;
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
        Relation {
            header: new_header,
            rows: new,
        }
    }

    fn project(mut self, vars: &Vec<String>) -> Self {
        let mut var2idx: HashMap<String, usize> = self
            .header
            .iter()
            .enumerate()
            .map(|(i, s)| (s.to_string(), i))
            .collect();

        let mut swaps: Vec<(usize, usize)> = Vec::new();
        for (idx, var) in vars.iter().enumerate() {
            let sidx = *skip_none!(var2idx.get(var));

            // if var is already in place, continue
            if idx == sidx {
                continue;
            }
            // else, swap var into that location
            swaps.push((idx, sidx));
            let displaced_var = skip_none!(self.header.get(idx)).to_string();
            var2idx.insert(var.to_string(), idx);
            var2idx.insert(displaced_var.clone(), sidx);
            self.header[idx] = var.to_string();
            self.header[sidx] = displaced_var;
        }

        println!("projection {:?} {:?}", self.header, swaps);
        for row in self.rows.iter_mut() {
            for (i1, i2) in swaps.iter() {
                row.swap(*i1, *i2);
            }
            row.truncate(vars.len());
        }

        Relation {
            header: vars.to_vec(),
            rows: self.rows,
        }
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
        Context {
            graph: graph,
            relations: Vec::new(),
        }
    }

    pub fn with_subject(&mut self, s: &Node, p: String, o: String) {
        let triples = Relation::from(
            self.graph.graph.get_triples_with_subject(s),
            vec!["_".to_string(), p, o],
        );
        self.relations.push(triples);
    }

    pub fn with_object(&mut self, s: String, p: String, o: &Node) {
        let triples = Relation::from(
            self.graph.graph.get_triples_with_object(o),
            vec![s, p, "_".to_string()],
        );
        self.relations.push(triples);
    }

    pub fn with_predicate(&mut self, s: String, p: &Node, o: String) {
        let triples = Relation::from(
            self.graph.graph.get_triples_with_predicate(p),
            vec![s, "_".to_string(), o],
        );
        self.relations.push(triples);
    }

    pub fn with_predicate_plus(&mut self, s: String, p: &Node, o: String) {
        self.transitive_closure(s, p, o);
    }

    pub fn with_subject_object(&mut self, s: &Node, p: String, o: &Node) {
        let triples = Relation::from(
            self.graph.graph.get_triples_with_subject_and_object(s, o),
            vec!["_".to_string(), p, "_".to_string()],
        );
        self.relations.push(triples);
    }

    pub fn with_subject_predicate(&mut self, s: &Node, p: &Node, o: String) {
        let triples = Relation::from(
            self.graph
                .graph
                .get_triples_with_subject_and_predicate(s, p),
            vec!["_".to_string(), "_".to_string(), o],
        );
        self.relations.push(triples);
    }

    pub fn with_predicate_object(&mut self, s: String, p: &Node, o: &Node) {
        let triples = Relation::from(
            self.graph.graph.get_triples_with_predicate_and_object(p, o),
            vec![s, "_".to_string(), "_".to_string()],
        );
        self.relations.push(triples);
    }

    // TODO: not pub
    pub fn transitive_closure(&mut self, s: String, p: &Node, o: String) {
        let mut iter = Iteration::new();

        let mut lookup: HashMap<u32, &Node> = HashMap::new();
        let triples: Vec<(u32, u32)> = self
            .graph
            .graph
            .get_triples_with_predicate(p)
            .iter()
            .map(|t| {
                let o_ = t.object();
                let o_hash = farmhash::hash32(node_to_string(o_)) as u32;
                lookup.insert(o_hash, o_);

                let s_ = t.subject();
                let s_hash = farmhash::hash32(node_to_string(s_)) as u32;
                lookup.insert(s_hash, s_);
                (s_hash, o_hash)
            })
            .collect();

        // transitive closure evalutes:
        // reachable(X, Z) :- reachable(X, Y), edge(Y, Z) .
        let reachable = iter.variable::<(u32, u32)>("reachable");
        let edge = iter.variable::<(u32, u32)>("edge");

        //// key is (O, S)
        edge.extend(triples.clone());
        reachable.extend(triples);
        while iter.changed() {
            reachable.from_join(&reachable, &edge, |_b, a, c| (c.clone(), a.clone()));
        }

        let rows: Vec<Vec<&Node>> = reachable
            .complete()
            .iter()
            .map(|(a, b)| vec![*lookup.get(a).unwrap(), *lookup.get(b).unwrap()])
            .collect();

        let rel = Relation {
            header: vec![s, "_".to_string(), o],
            rows: rows,
        };
        self.relations.push(rel);
    }

    pub fn joinall(mut self) -> Relation<'a> {
        let acc = self.relations.remove(0);
        self.relations
            .into_iter()
            .fold(acc, |a, b| Relation::join(a, b))
    }

    pub fn resolve(mut self) -> Option<Relation<'a>> {
        match self.relations.len() {
            0 => None,
            1 => Some(self.relations.remove(0)),
            _ => Some(self.joinall()),
        }
    }
}
