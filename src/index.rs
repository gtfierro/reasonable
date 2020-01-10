use std::collections::HashMap;
use fasthash::city;
use crate::common::{URI, Triple};

pub struct URIIndex {
    map : HashMap<URI, String>
}

impl URIIndex {
    pub fn new() -> Self {
        let mut idx = URIIndex {
            map: HashMap::new(),
        };
        idx.map.insert(0, "_".to_string());
        idx
    }

    pub fn put(&mut self, key: String) -> URI {
        let h = hash(&key);
        self.map.insert(h, key);
        h
    }

    pub fn put_str(&mut self, _key: &'static str) -> URI {
        let key = _key.to_string();
        let h = hash(&key);
        self.map.insert(h, key);
        h
    }

    pub fn get(&self, key: URI) -> Option<&String> {
        self.map.get(&key)
    }

    pub fn load_triples(&mut self, triples: Vec<(&'static str, &'static str, &'static str)>) -> Vec<Triple> {
        triples.iter().map(|(s, p, o)| {
            (self.put_str(s), (self.put_str(p), self.put_str(o)))
        }).collect()
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
