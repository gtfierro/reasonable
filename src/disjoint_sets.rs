extern crate datafrog;
extern crate disjoint_sets;

use disjoint_sets::UnionFind;
use datafrog::Iteration;
use crate::index::URIIndex;
use crate::common::{URI, Triple};
use std::collections::HashMap;

pub struct DisjointSets {
    lists: HashMap<URI, Vec<URI>>,
    uri2idx: HashMap<URI, usize>,
    idx2uri: Vec<URI>,
    ds: UnionFind,
}

impl DisjointSets {
    pub fn new(input: &Vec<Triple>) -> Self {
        let mut lists: HashMap<URI, Vec<URI>> = HashMap::new();
        // keeps track of data associated with each list element
        let mut values: HashMap<URI, URI> = HashMap::new();
        // disjoint set data structure
        let mut uri2idx: HashMap<URI, usize> = HashMap::new();
        let mut idx2uri: Vec<URI> = Vec::with_capacity(input.len());
        let mut counter: usize = 0;
        input.iter().map(|val| {
            let (a, (b, c)) = *val;
            vec![a, b, c].iter().map(|e| {
                let index = uri2idx.entry(*e).or_insert(counter);
                if *index == counter {
                    idx2uri.push(*e);
                    counter += 1;
                    // println!("uri2idx {} => {}", *e, index);
                }
            }).count();
        }).count();
        let mut ds = UnionFind::new(counter);

        // index lets us map u64s to the actual URIs
        let mut index = URIIndex::new();
        let rdffirst_node = index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#first");
        let rdfrest_node = index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest");
        let rdfnil_node = index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil");

        // data structures for building up the "first" and "rest" lists
        let mut list_iter = Iteration::new();
        let rdffirst = list_iter.variable::<(URI, ())>("rdffirst");
        rdffirst.extend(vec![(rdffirst_node, ())]);
        let rdfrest = list_iter.variable::<(URI, ())>("rdfrest");
        rdfrest.extend(vec![(rdfrest_node, ())]);
        let list_triples = list_iter.variable::<Triple>("list_triples");
        list_triples.extend(input.iter().map(|&(s, (p, o))| (p, (s, o))));
        let rdf_firsts = list_iter.variable::<()>("rdf_firsts");
        let rdf_rests = list_iter.variable::<()>("rdf_rests");

        while list_iter.changed() {
            rdf_firsts.from_join(&list_triples, &rdffirst, |&_, &(key, value), &_| { 
                values.insert(key, value);        
                ()
            });
            rdf_rests.from_join(&list_triples, &rdfrest, |&_, &(head, tail), &_| { 
                if tail == rdfnil_node {
                    return ()
                }
                let hn = uri2idx.get(&head).unwrap();
                let tn = uri2idx.get(&tail).unwrap();
                ds.union(*hn, *tn);
                // the head and tail are of each list segment
            });
        }
        values.into_iter().map(|(key, value)| {
            // go from key name to its uri2idx 
            let idx = uri2idx.get(&key).unwrap();
            // consult disjoint sets to get which set we are in
            let set = ds.find(*idx);
            // get the uri for the list
            let listname = idx2uri[set];
            // println!("key {} belongs to {} ({})", key, set, listname);
            let list = lists.entry(listname).or_insert(Vec::new());
            list.push(value);
        }).count();
        // lists.iter().map(|(k, v)| {
        //     println!("Set {} has {:?}", k, v);
        // }).count();
        // println!("Disjoint sets: {:?}", ds.to_vec());
        DisjointSets{
            lists: lists,
            uri2idx: uri2idx,
            idx2uri: idx2uri,
            ds: ds,
        }
    }

    pub fn get_list_values(&self, head: URI) -> Option<Vec<URI>> {
        if let Some(idx) = self.uri2idx.get(&head) {
            let realhead = self.idx2uri[self.ds.find(*idx)];
            if let Some(v) = self.lists.get(&realhead) {
                return Some(v.iter().cloned().collect());
            }
        }
        None
    }
}
