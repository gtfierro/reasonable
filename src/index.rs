use crate::common::{Triple, URI};
use fasthash::city;
use rdf::node::Node;
use rdf::uri::Uri;
use std::collections::HashMap;

pub struct URIIndex {
    map: HashMap<URI, Node>,
}

impl URIIndex {
    pub fn new() -> Self {
        let mut idx = URIIndex {
            map: HashMap::new(),
        };
        idx.map.insert(
            0,
            Node::UriNode {
                uri: Uri::new("_".to_string()),
            },
        );
        idx
    }

    pub fn put(&mut self, key: Node) -> URI {
        let s: String = match key {
            Node::UriNode { ref uri } => uri.to_string().clone(),
            Node::LiteralNode {
                ref literal,
                data_type: _,
                language: _,
            } =>  literal.to_string(),
            Node::BlankNode { ref id } => format!("_:{}", id.to_string()),
        };
        let h = hash(&s);
        self.map.insert(h, key);
        h
    }

    pub fn put_str(&mut self, _key: &'static str) -> URI {
        let key = _key.to_string();
        let h = hash(&key);
        self.map.insert(h, Node::UriNode { uri: Uri::new(key) });
        h
    }

    pub fn get(&self, key: URI) -> Option<&Node> {
        self.map.get(&key)
    }

    pub fn load_triples(
        &mut self,
        triples: Vec<(&'static str, &'static str, &'static str)>,
    ) -> Vec<Triple> {
        triples
            .iter()
            .map(|(s, p, o)| (self.put_str(s), (self.put_str(p), self.put_str(o))))
            .collect()
    }
}

pub fn hash(key: &String) -> URI {
    city::hash32(key)
}

#[allow(dead_code)]
pub fn hash_str(key: &'static str) -> URI {
    let s = key.to_string();
    city::hash32(&s)
}
