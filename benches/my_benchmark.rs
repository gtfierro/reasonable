use criterion::{black_box, criterion_group, criterion_main, Criterion};
use reasonable::owl::Reasoner;

#[inline]
pub fn brick1() {
    let mut r = Reasoner::new();
    r.load_file("example_models/ontologies/Brick.n3").unwrap();
    r.load_file("example_models/small1.n3").unwrap();
    r.reason();
}

#[inline]
pub fn brick2() {
    let mut r = Reasoner::new();
    r.load_file("example_models/ontologies/Brick.n3").unwrap();
    r.load_file("example_models/ontologies/owl.n3").unwrap();
    r.load_file("example_models/small1.n3").unwrap();
    r.reason();
}

#[inline]
pub fn brick3() {
    let mut r = Reasoner::new();
    r.load_file("example_models/ontologies/Brick.n3").unwrap();
    r.load_file("example_models/ontologies/rdfs.ttl").unwrap();
    r.load_file("example_models/small1.n3").unwrap();
    r.reason();
}

#[inline]
pub fn brick4() {
    let mut r = Reasoner::new();
    r.load_file("example_models/ontologies/Brick.n3").unwrap();
    r.load_file("example_models/ontologies/owl.n3").unwrap();
    r.load_file("example_models/ontologies/rdfs.ttl").unwrap();
    r.load_file("example_models/small1.n3").unwrap();
    r.reason();
}

pub fn bench_brick1(c: &mut Criterion) {
    c.bench_function("brick1", |b| b.iter(|| brick1() ));
}
pub fn bench_brick2(c: &mut Criterion) {
    c.bench_function("brick2", |b| b.iter(|| brick2() ));
}
pub fn bench_brick3(c: &mut Criterion) {
    c.bench_function("brick3", |b| b.iter(|| brick3() ));
}
pub fn bench_brick4(c: &mut Criterion) {
    c.bench_function("brick4", |b| b.iter(|| brick4() ));
}

fn setup() -> Criterion {
    Criterion::default().sample_size(20).with_plots()
}

criterion_group! {
    name=benches ;
    config=setup() ;
    targets=bench_brick1, bench_brick2, bench_brick3, bench_brick4
}
criterion_main!(benches);
