use std::collections::HashMap;
use fasthash::city;
use crate::types::URI;

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
}

pub fn hash(key: &String) -> URI {
    city::hash64(key)
}

#[allow(dead_code)] 
pub fn hash_str(key: &'static str) -> URI {
    let s = key.to_string();
    city::hash64(&s)
}

