//! Synthetic property-chain performance matrix.
//!
//! Run the default implementation with:
//! `cargo bench -p reasonable --bench property_chain`
//!
//! To measure list-indexing overhead with property-chain reasoning disabled:
//! `cargo bench -p reasonable --no-default-features --bench property_chain`

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use oxrdf::{NamedNode, Term, Triple};
use reasonable::reasoner::Reasoner;
use std::time::Duration;

const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const OWL_UNION: &str = "http://www.w3.org/2002/07/owl#unionOf";
const OWL_CHAIN: &str = "http://www.w3.org/2002/07/owl#propertyChainAxiom";

fn triple(s: impl AsRef<str>, p: &str, o: impl AsRef<str>) -> Triple {
    Triple::new(
        NamedNode::new_unchecked(s.as_ref()),
        NamedNode::new_unchecked(p),
        Term::NamedNode(NamedNode::new_unchecked(o.as_ref())),
    )
}

fn list_triples(prefix: &str, values: &[String], out: &mut Vec<Triple>) -> String {
    let cells: Vec<String> = (0..values.len())
        .map(|i| format!("urn:{prefix}:cell:{i}"))
        .collect();

    for (i, value) in values.iter().enumerate() {
        out.push(triple(&cells[i], RDF_FIRST, value));
        out.push(triple(
            &cells[i],
            RDF_REST,
            if i + 1 == cells.len() {
                RDF_NIL.to_string()
            } else {
                cells[i + 1].clone()
            },
        ));
    }
    cells[0].clone()
}

/// Many ordinary RDF lists, but no property-chain axioms. This isolates the
/// feature-enabled indexing overhead for list-heavy ontologies.
fn unrelated_lists(list_count: usize, list_len: usize) -> Vec<Triple> {
    let mut triples = Vec::with_capacity(list_count * (2 * list_len + 1));
    for i in 0..list_count {
        let values: Vec<String> = (0..list_len)
            .map(|j| format!("urn:class:{i}:{j}"))
            .collect();
        let head = list_triples(&format!("union:{i}"), &values, &mut triples);
        triples.push(triple(format!("urn:union:{i}"), OWL_UNION, head));
    }
    triples
}

/// Many chains sharing one suffix. This exercises path reuse without making
/// the benchmark dominated by a large branching result set.
fn shared_suffix(chain_count: usize, suffix_len: usize) -> Vec<Triple> {
    let mut triples = Vec::new();
    let suffix: Vec<String> = (0..suffix_len)
        .map(|j| format!("urn:p:suffix:{j}"))
        .collect();
    let suffix_head = list_triples("shared-suffix", &suffix, &mut triples);

    for i in 0..chain_count {
        let head = format!("urn:chain:{i}:head");
        let first = format!("urn:p:first:{i}");
        triples.push(triple(format!("urn:super:{i}"), OWL_CHAIN, &head));
        triples.push(triple(&head, RDF_FIRST, &first));
        triples.push(triple(&head, RDF_REST, &suffix_head));

        let mut from = format!("urn:node:{i}:0");
        let first_to = format!("urn:node:{i}:1");
        triples.push(triple(&from, &first, &first_to));
        from = first_to;
        for (j, property) in suffix.iter().enumerate() {
            let to = format!("urn:node:{i}:{}", j + 2);
            triples.push(triple(&from, property, &to));
            from = to;
        }
    }
    triples
}

/// A single chain over a branching layered graph. The output grows as
/// fanout^chain_len, making result explosion visible in benchmark output.
fn branching_chain(chain_len: usize, fanout: usize) -> Vec<Triple> {
    let mut triples = Vec::new();
    let properties: Vec<String> = (0..chain_len)
        .map(|i| format!("urn:p:branch:{i}"))
        .collect();
    let head = list_triples("branching", &properties, &mut triples);
    triples.push(triple("urn:super:branching", OWL_CHAIN, head));

    let mut layer = vec!["urn:branch:root".to_string()];
    for (hop, property) in properties.iter().enumerate() {
        let mut next = Vec::with_capacity(layer.len() * fanout);
        for source in &layer {
            for branch in 0..fanout {
                let target = format!("urn:branch:{hop}:{source}:{branch}");
                triples.push(triple(source, property, &target));
                next.push(target);
            }
        }
        layer = next;
    }
    triples
}

fn incremental_final_hop(chain_len: usize) -> (Reasoner, Triple) {
    let properties: Vec<String> = (0..chain_len)
        .map(|i| format!("urn:p:incremental:{i}"))
        .collect();
    let mut initial = Vec::new();
    let head = list_triples("incremental", &properties, &mut initial);
    initial.push(triple("urn:super:incremental", OWL_CHAIN, head));

    let mut from = "urn:inc:0".to_string();
    for (i, property) in properties.iter().enumerate().take(chain_len - 1) {
        let to = format!("urn:inc:{i}", i = i + 1);
        initial.push(triple(&from, property, &to));
        from = to;
    }

    let final_hop = triple(&from, &properties[chain_len - 1], "urn:inc:final");
    let mut reasoner = Reasoner::new();
    reasoner.load_triples(initial);
    reasoner.reason();
    (reasoner, final_hop)
}

fn bench_full(c: &mut Criterion, name: &str, input: Vec<Triple>) {
    c.bench_function(name, |b| {
        b.iter_batched(
            || {
                let mut reasoner = Reasoner::new();
                reasoner.load_triples(input.clone());
                reasoner
            },
            |mut reasoner| {
                reasoner.reason();
                std::hint::black_box(reasoner.get_triples().len());
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_incremental(c: &mut Criterion, name: &str, chain_len: usize) {
    c.bench_function(name, |b| {
        b.iter_batched(
            || incremental_final_hop(chain_len),
            |(mut reasoner, final_hop)| {
                reasoner.load_triples(vec![final_hop]);
                reasoner.reason();
                std::hint::black_box(reasoner.get_triples().len());
            },
            BatchSize::SmallInput,
        );
    });
}

fn property_chain_benchmarks(c: &mut Criterion) {
    bench_full(c, "no_chain_500_lists_x8", unrelated_lists(500, 8));
    bench_full(c, "no_chain_1500_lists_x8", unrelated_lists(1500, 8));
    bench_full(c, "shared_suffix_100_chains_x8", shared_suffix(100, 8));
    bench_full(c, "shared_suffix_500_chains_x8", shared_suffix(500, 8));
    bench_full(c, "branching_5_hops_x4", branching_chain(5, 4));
    bench_full(c, "branching_7_hops_x4", branching_chain(7, 4));
    bench_incremental(c, "incremental_final_hop_8", 8);
    bench_incremental(c, "incremental_final_hop_16", 16);
    bench_incremental(c, "incremental_final_hop_64", 64);
    bench_incremental(c, "incremental_final_hop_128", 128);
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(3))
        .sample_size(20);
    targets = property_chain_benchmarks
}

criterion_main!(benches);
