use reasonable::reasoner::Reasoner;

use log::info;
use std::env;
use std::time::Instant;

fn main() {
    env_logger::init();
    let mut r = Reasoner::new();
    env::args()
        .skip(1)
        .map(|filename| {
            info!("Loading file {}", &filename);
            r.load_file(&filename).unwrap()
        })
        .count();
    let reasoning_start = Instant::now();
    info!("Starting reasoning");
    r.reason();
    info!(
        "Reasoning completed in {:.02}sec",
        reasoning_start.elapsed().as_secs_f64()
    );
    r.dump_file("output.ttl").unwrap();
}
