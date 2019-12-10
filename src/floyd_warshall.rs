extern crate datafrog;

use datafrog::{Iteration, Variable};
use itertools::Itertools;
use std::time::Instant;
use crate::index::{URIIndex, hash_str};
use crate::types::{URI, Triple};
use std::collections::HashMap;
use roaring::RoaringBitmap;

const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

pub struct FloydWarshall {
    reachable: HashMap<URI, RoaringBitmap>,
}

impl FloydWarshall {
    pub fn new(input: &Vec<Triple>) -> Self {
        let mut reachable: HashMap<URI, RoaringBitmap> = HashMap::new();
        let mut dist: HashMap<(URI, URI), bool> = HashMap::new();
        let mut values: HashMap<URI, URI> = HashMap::new();

        let mut index = URIIndex::new();
        let rdffirst_node = index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#first");
        let rdfrest_node = index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest");
        let rdfnil_node = index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil");

        let mut list_iter = Iteration::new();

        let rdffirst = list_iter.variable::<(URI, ())>("rdffirst");
        rdffirst.extend(vec![(rdffirst_node, ())]);

        let rdfrest = list_iter.variable::<(URI, ())>("rdfrest");
        rdfrest.extend(vec![(rdfrest_node, ())]);

        let mut list_triples = list_iter.variable::<Triple>("list_triples");
        list_triples.extend(input.iter().map(|&(s, (p, o))| (p, (s, o))));

        let rdf_firsts = list_iter.variable::<()>("rdf_firsts");
        let rdf_rests = list_iter.variable::<()>("rdf_rests");
        let mut tails = RoaringBitmap::new();
        while list_iter.changed() {
            rdf_firsts.from_join(&list_triples, &rdffirst, |&_, &(key, value), &_| { 
                values.insert(key, value);        
                dist.insert((key, key), true);
                ()
            });
            rdf_rests.from_join(&list_triples, &rdfrest, |&_, &(p1, p2), &_| { 
                if p2 == rdfnil_node {
                    return ()
                }
                tails.insert(p2);
                dist.insert((p1, p2), true);
                dist.insert((p2, p1), true);
                let mut rb_p1 = reachable.entry(p1).or_insert(RoaringBitmap::new());
                rb_p1.insert(p1);
                rb_p1.insert(p2);
                let mut rb_p2 = reachable.entry(p2).or_insert(RoaringBitmap::new());
                rb_p2.insert(p1);
                rb_p2.insert(p2);
                ()
            });
        }

        println!("# values: {}", values.len());
        let combinations: Vec<(URI, URI)> = values.keys().combinations(2).map(|v| (**v.get(0).unwrap(), **v.get(1).unwrap())).collect();
        for (idx, k) in values.keys().enumerate() {
            let mut t0 = Instant::now();
            for (i, j) in combinations.iter() {
                    if k == i && i == j { continue }
                    let i_to_k = dist.get(&(*i,*k)).unwrap_or(&false);
                    let k_to_j = dist.get(&(*k,*j)).unwrap_or(&false);
                    if *i_to_k && *k_to_j {
                        dist.insert((*i, *j), true);
                        let mut rb = reachable.entry(*i).or_insert(RoaringBitmap::new());
                        rb.insert(*i);
                        rb.insert(*j);
                        let mut rb = reachable.entry(*j).or_insert(RoaringBitmap::new());
                        rb.insert(*i);
                        rb.insert(*j);
                    }
            }
            println!("Finished iteration {} in {:?}", idx, t0.elapsed());
        }

        for k in values.keys() {
            if tails.contains(*k) {
                reachable.remove(k);
            }
        }

        FloydWarshall{
            reachable: reachable,
        }
    }

    pub fn get_list_values(&self, head: URI) -> Option<Vec<URI>> {
        if let Some(rb) = self.reachable.get(&head) {
            return Some(rb.iter().collect());
        }
        None
    }
}

#[test]
fn test_fw_1() -> Result<(), String> {
    let mut index = URIIndex::new();
    let triples = vec![
        ("A", RDF_FIRST, "val1"),
        ("A", RDF_REST, "B"),
        ("B", RDF_FIRST, "val2"),
        ("B", RDF_REST, RDF_NIL),

        ("C", RDF_FIRST, "val11"),
        ("C", RDF_REST, "D"),
        ("D", RDF_FIRST, "val12"),
        ("D", RDF_REST, "E"),
        ("E", RDF_FIRST, "val13"),
        ("E", RDF_REST, "F"),
        ("F", RDF_FIRST, "val14"),
        ("F", RDF_REST, RDF_NIL),
    ];
    let trips = index.load_triples(triples);
    let fw = FloydWarshall::new(&trips);
    assert!(fw.get_list_values(hash_str("A")).is_some());
    assert!(fw.get_list_values(hash_str("B")).is_none());
    assert!(fw.get_list_values(hash_str("A")).unwrap().len() == 2);
    assert!(fw.get_list_values(hash_str("C")).unwrap().len() == 4);
    assert!(fw.get_list_values(hash_str("D")).is_none());
    assert!(fw.get_list_values(hash_str("E")).is_none());
    assert!(fw.get_list_values(hash_str("F")).is_none());
    Ok(())
}
