use clap::Parser;
use reasonable::reasoner::Reasoner;
use std::path::PathBuf;

use env_logger::Env;
use log::info;

use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "reasonable", author, version, about)]
struct Cli {
    #[arg(required = true)]
    input_files: Vec<PathBuf>,
    #[arg(short, long, default_value_os_t = PathBuf::from("output.ttl"))]
    output_file: PathBuf,
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    let mut r = Reasoner::new();

    cli.input_files
        .into_iter()
        .map(|filename| {
            info!("Loading file {}", filename.display());
            let file_name = filename.as_path().to_str().unwrap();
            r.load_file(file_name).unwrap()
        })
        .for_each(drop);
    let reasoning_start = Instant::now();
    info!("Starting reasoning");
    r.reason();
    info!(
        "Reasoning completed in {:.02}sec",
        reasoning_start.elapsed().as_secs_f64()
    );
    info!("Writing to {}", cli.output_file.to_str().unwrap());
    r.dump_file(cli.output_file.to_str().unwrap()).unwrap();
}
