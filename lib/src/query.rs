#![cfg(feature = "legacy-query")]

use datafrog::Iteration;
use farmhash;
use oxigraph::model::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_sexpr::from_str;
use std::collections::HashMap;
use std::fmt;

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
    Iri(NamedNode),
}

#[derive(Serialize, Deserialize, Debug)]
struct Query {
    prefixes: Vec<[String; 2]>,
    select: Vec<String>,
    clauses: Vec<[String; 3]>,
}

impl Query {
    fn to_atom(&self, s: &str) => Atom {
        if s.starts_with('?') {
            Atom::Var(s.to_string())
        } else {
            for pfx in &self.prefixes {
                let npfx = format!("{}:", pfx[0]);
                if s.starts_with(&npfx) {
                    let v: &str = s.split(':').nth(1).unwrap();
                    return Atom::Iri(NamedNode::new(format!("{}{}", pfx[1], v)).unwrap());
                }
            }
            Atom::Iri(NamedNode::new(s).unwrap())
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
    graph: Graph as OxiGraph,
}

impl From<Vec<(NamedNode, NamedNode, NamedNode)>> for Graph {
    fn from(triples: Vec<(NamedNode, NamedNode, NamedNode)>) -> Self {
        let mut graph = OxiGraph::default();
        for (s, p, o) in triples {
            graph.insert(TripleRef::new(SubjectRef::NamedNode(s.as_ref()), p.as_ref(), TermRef::NamedNode(o.as_ref())));
        }
        Graph { graph }
    }
}

impl Graph {
    pub fn query(&self, q: String) -> Option<Relation> {
        let re = Regex::new(r"([\t\n\r]+)|(  +)").unwrap();
        let q = re.replace_all(&q, "");
        let query = from_str::<Query>(&q).unwrap();
        let mut ctx = Context::new(self);
        let clauses = query.parse();
        for clause in clauses {
            match clause {
                [Atom::Var(s), Atom::Iri(p), Atom::Iri(o)] => {
                    ctx.with_predicate_object(s, &p, &o)
                }
                [Atom::Iri(s), Atom::Var(p), Atom::Iri(o)] => ctx.with_subject_object(&s, p, &o),
                [Atom::Iri(s), Atom::Iri(p), Atom::Var(o)] => {
                    ctx.with_subject_predicate(&s, &p, o)
                }
                [Atom::Iri(s), Atom::Var(p), Atom::Var(o)] => ctx.with_subject(&s, p, o),
                [Atom::Var(s), Atom::Iri(p), Atom::Var(o)] => ctx.with_predicate(s, &p, o),
                [Atom::Var(s), Atom::Var(p), Atom::Iri(o)] => ctx.with_object(s, p, &o),
                [Atom::Var(_), Atom::Var(_), Atom::Var(_)] => panic!("not covered"),
                [Atom::Iri(_), Atom::Iri(_), Atom::Iri(_)] => panic!("not covered"),
            };
        }
        ctx.resolve().map(|result| result.project(&query.select))
    }
}

pub struct Relation {
    header: Vec<String>,
    rows: Vec<Vec<Term>>,
}

impl Relation {
    fn from_triples(triples: Vec<Triple>, header: Vec<String>) -> Self {
        let mut rows = Vec::with_capacity(triples.len());
        for t in triples {
            let s_term = match t.subject {
                Subject::NamedNode(nn) => Term::NamedNode(nn),
                Subject::BlankNode(bn) => Term::BlankNode(bn),
                // No RDF-star support here
            };
            let p_term = Term::NamedNode(t.predicate);
            let o_term = t.object;
            rows.push(vec![s_term, p_term, o_term]);
        }
        Relation { header, rows }
    }

    fn join(first: Relation, other: Relation) -> Self {
        let mut new: Vec<Vec<Term>> = Vec::new();
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
                let mut row: Vec<Term> = Vec::new();
                for jh in &new_indices {
                    match jh {
                        JoinHeader::Left(idx) => row.push(t1.get(*idx).unwrap().clone()),
                        JoinHeader::Right(idx) => row.push(t2.get(*idx).unwrap().clone()),
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
            if idx == sidx {
                continue;
            }
            swaps.push((idx, sidx));
            let displaced_var = skip_none!(self.header.get(idx)).to_string();
            var2idx.insert(var.to_string(), idx);
            var2idx.insert(displaced_var.clone(), sidx);
            self.header[idx] = var.to_string();
            self.header[sidx] = displaced_var;
        }

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

impl fmt::Display for Relation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}\n", self.header)?;
        for t in self.rows.iter() {
            write!(f, "[")?;
            for term in t.iter() {
                write!(f, "{} ", term)?;
            }
            write!(f, "]\n")?;
        }
        Ok(())
    }
}

pub struct Context<'a> {
    graph: &'a Graph,
    relations: Vec<Relation>,
}

impl<'a> Context<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Context {
            graph,
            relations: Vec::new(),
        }
    }

    pub fn with_subject(&mut self, s: &NamedNode, p: String, o: String) {
        let triples: Vec<Triple> = self
            .graph
            .graph
            .triples_for_subject(s.as_ref())
            .map(|t| t.into())
            .collect();
        let rel = Relation::from_triples(triples, vec!["_".to_string(), p, o]);
        self.relations.push(rel);
    }

    pub fn with_object(&mut self, s: String, p: String, o: &NamedNode) {
        let triples: Vec<Triple> = self
            .graph
            .graph
            .triples_for_object(TermRef::NamedNode(o.as_ref()))
            .map(|t| t.into())
            .collect();
        let rel = Relation::from_triples(triples, vec![s, p, "_".to_string()]);
        self.relations.push(rel);
    }

    pub fn with_predicate(&mut self, s: String, p: &NamedNode, o: String) {
        let triples: Vec<Triple> = self
            .graph
            .graph
            .triples_for_predicate(p.as_ref())
            .map(|t| t.into())
            .collect();
        let rel = Relation::from_triples(triples, vec![s, "_".to_string(), o]);
        self.relations.push(rel);
    }

    pub fn with_predicate_plus(&mut self, s: String, p: &NamedNode, o: String) {
        self.transitive_closure(s, p, o);
    }

    pub fn with_subject_object(&mut self, s: &NamedNode, p: String, o: &NamedNode) {
        let triples: Vec<Triple> = self
            .graph
            .graph
            .triples_for_subject(s.as_ref())
            .filter(|t| t.object == TermRef::NamedNode(o.as_ref()))
            .map(|t| t.into())
            .collect();
        let rel = Relation::from_triples(triples, vec!["_".to_string(), p, "_".to_string()]);
        self.relations.push(rel);
    }

    pub fn with_subject_predicate(&mut self, s: &NamedNode, p: &NamedNode, o: String) {
        let triples: Vec<Triple> = self
            .graph
            .graph
            .triples_for_subject(s.as_ref())
            .filter(|t| t.predicate == p.as_ref())
            .map(|t| t.into())
            .collect();
        let rel = Relation::from_triples(triples, vec!["_".to_string(), "_".to_string(), o]);
        self.relations.push(rel);
    }

    pub fn with_predicate_object(&mut self, s: String, p: &NamedNode, o: &NamedNode) {
        let triples: Vec<Triple> = self
            .graph
            .graph
            .triples_for_predicate(p.as_ref())
            .filter(|t| t.object == TermRef::NamedNode(o.as_ref()))
            .map(|t| t.into())
            .collect();
        let rel = Relation::from_triples(triples, vec![s, "_".to_string(), "_".to_string()]);
        self.relations.push(rel);
    }

    pub fn transitive_closure(&mut self, s: String, p: &NamedNode, o: String) {
        let mut iter = Iteration::new();

        let mut lookup: HashMap<u64, Term> = HashMap::new();
        let edges: Vec<(u64, u64)> = self
            .graph
            .graph
            .triples_for_predicate(p.as_ref())
            .map(|t| {
                let s_term = match t.subject {
                    SubjectRef::NamedNode(nn) => Term::NamedNode(nn.into_owned()),
                    SubjectRef::BlankNode(bn) => Term::BlankNode(bn.into_owned()),
                };
                let o_term = match t.object {
                    TermRef::NamedNode(nn) => Term::NamedNode(nn.into_owned()),
                    TermRef::BlankNode(bn) => Term::BlankNode(bn.into_owned()),
                    TermRef::Literal(l) => Term::Literal(l.into_owned()),
                };
                let s_hash = farmhash::hash64(&s_term.to_string());
                let o_hash = farmhash::hash64(&o_term.to_string());
                lookup.insert(s_hash, s_term);
                lookup.insert(o_hash, o_term);
                (s_hash, o_hash)
            })
            .collect();

        let reachable = iter.variable::<(u64, u64)>("reachable");
        let edge = iter.variable::<(u64, u64)>("edge");

        edge.extend(edges.clone());
        reachable.extend(edges);
        while iter.changed() {
            reachable.from_join(&reachable, &edge, |_b, a, c| (c.clone(), a.clone()));
        }

        let rows: Vec<Vec<Term>> = reachable
            .complete()
            .iter()
            .map(|(a, b)| vec![lookup.get(a).unwrap().clone(), Term::NamedNode(p.clone()), lookup.get(b).unwrap().clone()])
            .collect();

        let rel = Relation {
            header: vec![s, "_".to_string(), o],
            rows,
        };
        self.relations.push(rel);
    }

    pub fn joinall(mut self) -> Relation {
        let acc = self.relations.remove(0);
        self.relations
            .into_iter()
            .fold(acc, |a, b| Relation::join(a, b))
    }

    pub fn resolve(mut self) -> Option<Relation> {
        match self.relations.len() {
            0 => None,
            1 => Some(self.relations.remove(0)),
            _ => Some(self.joinall()),
        }
    }
}
