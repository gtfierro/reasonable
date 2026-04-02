use criterion::{criterion_group, criterion_main, Criterion, PlotConfiguration};
use reasonable::manager::parse_file;
use reasonable::reasoner::Reasoner;

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
        b.iter_with_setup(
            || {
                setup_reasoner![
                    "example_models/ontologies/Brick.n3",
                    "example_models/small1.n3"
                ]
            },
            |mut r| r.reason(),
        )
    });

    group.bench_function("brick+owl_small1", move |b| {
        b.iter_with_setup(
            || {
                setup_reasoner![
                    "example_models/ontologies/Brick.n3",
                    "example_models/ontologies/owl.n3",
                    "example_models/small1.n3"
                ]
            },
            |mut r| r.reason(),
        )
    });

    group.bench_function("brick+rdfs_small1", move |b| {
        b.iter_with_setup(
            || {
                setup_reasoner![
                    "example_models/ontologies/Brick.n3",
                    "example_models/ontologies/rdfs.ttl",
                    "example_models/small1.n3"
                ]
            },
            |mut r| r.reason(),
        )
    });

    group.bench_function("brick+rdfs+owl_small1", move |b| {
        b.iter_with_setup(
            || {
                setup_reasoner![
                    "example_models/ontologies/Brick.n3",
                    "example_models/ontologies/owl.n3",
                    "example_models/ontologies/rdfs.ttl",
                    "example_models/small1.n3"
                ]
            },
            |mut r| r.reason(),
        )
    });

    group.finish()
}

fn bench_reload(c: &mut Criterion) {
    let plot_config = PlotConfiguration::default();
    let mut group = c.benchmark_group("reload_test");
    group.plot_config(plot_config);

    group.bench_function("brick_small1", move |b| {
        b.iter_with_setup(
            || {
                setup_reasoner![
                    "example_models/ontologies/Brick.n3",
                    "example_models/small1.n3"
                ]
            },
            |mut r| {
                r.reason();
                r.load_file("example_models/small1.n3").unwrap();
                r.reason();
            },
        )
    });

    group.bench_function("brick_soda", move |b| {
        b.iter_with_setup(
            || {
                setup_reasoner![
                    "example_models/ontologies/Brick.n3",
                    "example_models/soda_hall.n3"
                ]
            },
            |mut r| {
                r.reason();
                r.load_file("example_models/soda_hall.n3").unwrap();
                r.reason();
            },
        )
    });
}

fn bench_incremental(c: &mut Criterion) {
    let plot_config = PlotConfiguration::default();
    let mut group = c.benchmark_group("incremental_reason");
    group.plot_config(plot_config);

    // Benchmark: add a single triple incrementally after full materialization of Brick
    group.bench_function("brick_add_1_triple", move |b| {
        b.iter_with_setup(
            || {
                let mut r = setup_reasoner!["example_models/ontologies/Brick.n3"];
                r.reason(); // Full materialization
                r
            },
            |mut r| {
                r.load_triples_str(vec![(
                    "urn:test_sensor_1",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "https://brickschema.org/schema/Brick#Temperature_Sensor",
                )]);
                r.reason(); // Incremental
            },
        )
    });

    // Benchmark: full re-materialization for comparison
    group.bench_function("brick_full_rematerialization", move |b| {
        b.iter_with_setup(
            || {
                let mut r = setup_reasoner!["example_models/ontologies/Brick.n3"];
                r.load_triples_str(vec![(
                    "urn:test_sensor_1",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "https://brickschema.org/schema/Brick#Temperature_Sensor",
                )]);
                r
            },
            |mut r| {
                r.reason(); // Full materialization from scratch
            },
        )
    });

    // Benchmark: incremental with small1.n3 file (multiple triples)
    group.bench_function("brick_then_small1", move |b| {
        b.iter_with_setup(
            || {
                let mut r = setup_reasoner!["example_models/ontologies/Brick.n3"];
                r.reason(); // Full materialization
                r
            },
            |mut r| {
                r.load_file("example_models/small1.n3").unwrap();
                r.reason(); // Incremental
            },
        )
    });

    group.finish()
}

fn setup() -> Criterion {
    Criterion::default().sample_size(10).with_plots()
}

criterion_group! {
    name=benches;
    config=setup();
    targets=bench_simple, bench_reload, bench_incremental
}

criterion_main!(benches);
