use criterion::{black_box, criterion_group, criterion_main, Criterion, PlotConfiguration};
use reasonable::owl::Reasoner;

// setup_reasoner!("ontologies/Brick.n3", "small1.n3");
macro_rules! setup_reasoner {
    ( $( $file:expr ),* ) => {
        {
            let mut r = Reasoner::new();
            $(
                r.load_file(&format!("{}", $file).to_string()).unwrap();
            )*
            r
        }
    };
}

fn bench_simple(c: &mut Criterion) {
    let plot_config = PlotConfiguration::default();
    let mut group = c.benchmark_group("ontology_small1");
    group.plot_config(plot_config);

    group.bench_function("brick_small1", move |b| {
        b.iter_with_setup(|| setup_reasoner![
        "example_models/ontologies/Brick.n3",
        "example_models/small1.n3"
        ], |mut r| r.reason())
    });

    group.bench_function("brick+owl_small1", move |b| {
        b.iter_with_setup(|| setup_reasoner![
        "example_models/ontologies/Brick.n3",
        "example_models/ontologies/owl.n3",
        "example_models/small1.n3"
        ], |mut r| r.reason())
    });

    group.bench_function("brick+rdfs_small1", move |b| {
        b.iter_with_setup(|| setup_reasoner![
        "example_models/ontologies/Brick.n3",
        "example_models/ontologies/rdfs.ttl",
        "example_models/small1.n3"
        ], |mut r| r.reason())
    });

    group.bench_function("brick+rdfs+owl_small1", move |b| {
        b.iter_with_setup(|| setup_reasoner![
        "example_models/ontologies/Brick.n3",
        "example_models/ontologies/owl.n3",
        "example_models/ontologies/rdfs.ttl",
        "example_models/small1.n3"
        ], |mut r| r.reason())
    });

    group.finish()
}

fn setup() -> Criterion {
    Criterion::default().sample_size(50).with_plots()
}

criterion_group!{
    name=benches;
    config=setup();
    targets=bench_simple
}

criterion_main!(benches);
