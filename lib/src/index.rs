use crate::common::URI;

use oxrdf::{IriParseError, NamedNode, Term};
use std::collections::HashMap;

pub struct URIIndex {
    fwd: HashMap<Term, URI>,
    rev: Vec<Term>,
}

impl URIIndex {
    pub fn new() -> Self {
        // Reserve 0 as a sentinel to preserve existing behavior
        let mut rev = Vec::with_capacity(1024);
        let sentinel = Term::NamedNode(NamedNode::new_unchecked("urn:_".to_string()));
        rev.push(sentinel.clone());
        let mut fwd = HashMap::new();
        fwd.insert(sentinel, 0);
        URIIndex { fwd, rev }
    }

    pub fn put(&mut self, key: Term) -> URI {
        if let Some(&id) = self.fwd.get(&key) {
            return id;
        }
        let id = self.rev.len() as URI;
        self.rev.push(key.clone());
        self.fwd.insert(key, id);
        id
    }

    pub fn put_str(&mut self, key: &'static str) -> Result<URI, IriParseError> {
        let term = Term::NamedNode(NamedNode::new(key)?);
        Ok(self.put(term))
    }

    pub fn get(&self, key: URI) -> Option<&Term> {
        self.rev.get(key as usize)
    }
}
