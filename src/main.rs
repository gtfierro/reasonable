use reasonable::reasoner::Reasoner;
use std::path::PathBuf;
use clap::{Parser, Subcommand};


use log::info;
use std::env;
use std::time::Instant;

#[derive(Parser)]
#[command(name="reasonable")]
#[command(about="Performs OWL 2 RL Reasoning")]
struct Cli {
    input_files: Vec<PathBuf>,
    output_file: Optional<PathBuf>,
}


fn main() {
    env_logger::init();

    let cli = Cli::parse();
    print!("{:?}", cli);

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
    info!("Writing to output.ttl");
    r.dump_file("output.ttl").unwrap();
}
