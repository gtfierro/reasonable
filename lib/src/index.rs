use crate::common::URI;

use hashbrown::hash_table::{Entry, HashTable};
use oxrdf::{IriParseError, NamedNode, NamedNodeRef, Term, TermRef};
use std::hash::{BuildHasher, RandomState};

/// Interns RDF terms as dense `URI` ids.
///
/// Each term is stored once, in `terms`; the table holds only ids and is probed
/// with borrowed terms, so looking up a term that is already known allocates nothing.
pub struct URIIndex {
    hasher: RandomState,
    ids: HashTable<URI>,
    terms: Vec<Term>,
}

impl URIIndex {
    pub fn new() -> Self {
        let mut index = URIIndex {
            hasher: RandomState::new(),
            ids: HashTable::with_capacity(1024),
            terms: Vec::with_capacity(1024),
        };
        // Reserve 0 as a sentinel to preserve existing behavior
        index.put_ref(NamedNodeRef::new_unchecked("urn:_").into());
        index
    }

    pub fn put(&mut self, key: Term) -> URI {
        let hash = self.hasher.hash_one(key.as_ref());
        let URIIndex { hasher, ids, terms } = self;
        match ids.entry(
            hash,
            |&id| terms[id as usize] == key,
            |&id| hasher.hash_one(terms[id as usize].as_ref()),
        ) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let id = terms.len() as URI;
                terms.push(key);
                entry.insert(id);
                id
            }
        }
    }

    /// Like `put`, but only copies the term if it is new
    pub fn put_ref(&mut self, key: TermRef<'_>) -> URI {
        let hash = self.hasher.hash_one(key);
        let URIIndex { hasher, ids, terms } = self;
        match ids.entry(
            hash,
            |&id| terms[id as usize].as_ref() == key,
            |&id| hasher.hash_one(terms[id as usize].as_ref()),
        ) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let id = terms.len() as URI;
                terms.push(key.into_owned());
                entry.insert(id);
                id
            }
        }
    }

    pub fn put_str(&mut self, key: &'static str) -> Result<URI, IriParseError> {
        let term = Term::NamedNode(NamedNode::new(key)?);
        Ok(self.put(term))
    }

    pub fn get(&self, key: URI) -> Option<&Term> {
        self.terms.get(key as usize)
    }
}
