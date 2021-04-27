use crate::common::URI;
use fasthash::city;
use oxigraph::model::{Term, NamedNode, IriParseError};
use std::collections::HashMap;

pub struct URIIndex {
    map: HashMap<URI, Term>,
}

impl URIIndex {
    pub fn new() -> Self {
        let mut idx = URIIndex {
            map: HashMap::new(),
        };
        idx.map.insert(0, Term::NamedNode(NamedNode::new("urn:_").unwrap()));
        idx
    }

    pub fn put(&mut self, key: Term) -> URI {
        let h = hash(&key);
        self.map.insert(h, key);
        h
    }

    pub fn put_str(&mut self, _key: &'static str) -> Result<URI, IriParseError> {
        let key = Term::NamedNode(NamedNode::new(_key)?);
        let h = hash(&key);
        self.map.insert(h, key);
        Ok(h)
    }

    pub fn get(&self, key: URI) -> Option<&Term> {
        self.map.get(&key)
    }
}

pub fn hash(key: &Term) -> URI {
    city::hash32(key.to_string())
}

#[allow(dead_code)]
pub fn hash_str(key: &'static str) -> URI {
    let s = key.to_string();
    city::hash32(&s)
}
