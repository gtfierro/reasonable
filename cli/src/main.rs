use clap::{ArgAction, Parser, ValueEnum};
use reasonable::reasoner::Reasoner;
use std::path::PathBuf;

use env_logger::Env;
use log::info;

use std::time::Instant;

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ErrorFormat {
    Text,
    Json,
    Ndjson,
}

#[derive(Parser, Debug)]
#[command(name = "reasonable", author, version, about)]
struct Cli {
    #[arg(required = true)]
    input_files: Vec<PathBuf>,
    #[arg(short, long, default_value_os_t = PathBuf::from("output.ttl"))]
    output_file: PathBuf,
    /// How to print diagnostics (rule violations)
    #[arg(long, value_enum, default_value_t = ErrorFormat::Text)]
    error_format: ErrorFormat,
    /// Fail if any of these rule IDs occur (repeat or comma-separate; e.g., cax-dw,prp-pdw)
    #[arg(long, action = ArgAction::Append, value_delimiter = ',')]
    fail_on: Vec<String>,
    /// Maximum number of diagnostics to print
    #[arg(long)]
    max_diagnostics: Option<usize>,
    /// Only print a summary of diagnostics
    #[arg(long, action = ArgAction::SetTrue)]
    summary_only: bool,
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

    // Report diagnostics
    let diags = r.errors();
    let mut count = 0usize;
    let limit = cli.max_diagnostics.unwrap_or(usize::MAX);
    if !cli.summary_only {
        match cli.error_format {
            ErrorFormat::Text => {
                for d in diags.iter() {
                    if count >= limit {
                        break;
                    }
                    println!("{} {}: {}", d.code(), d.rule(), d.message());
                    count += 1;
                }
            }
            ErrorFormat::Json => {
                // print a single JSON array
                let mut first = true;
                print!("[");
                for d in diags.iter() {
                    if count >= limit {
                        break;
                    }
                    if !first {
                        print!(",");
                    }
                    first = false;
                    print!(
                        "{{\"code\":\"{}\",\"rule\":\"{}\",\"severity\":\"{}\",\"message\":\"{}\"}}",
                        d.code(),
                        d.rule(),
                        d.severity(),
                        escape_json(d.message())
                    );
                    count += 1;
                }
                println!("]");
            }
            ErrorFormat::Ndjson => {
                for d in diags.iter() {
                    if count >= limit {
                        break;
                    }
                    println!(
                        "{{\"code\":\"{}\",\"rule\":\"{}\",\"severity\":\"{}\",\"message\":\"{}\"}}",
                        d.code(),
                        d.rule(),
                        d.severity(),
                        escape_json(d.message())
                    );
                    count += 1;
                }
            }
        }
    }

    // Summary and exit code handling
    let total = diags.len();
    if cli.summary_only {
        println!("diagnostics: {}", total);
    }
    let mut should_fail = false;
    if !cli.fail_on.is_empty() {
        for d in diags.iter() {
            if matches_fail(&cli.fail_on, d.code(), d.rule()) {
                should_fail = true;
                break;
            }
        }
    }
    if should_fail {
        std::process::exit(2);
    }
}

fn matches_fail(fail_on: &[String], code: &str, rule: &str) -> bool {
    let code_upper = code.to_ascii_uppercase();
    let rule_lower = rule.to_ascii_lowercase();
    fail_on.iter().any(|s| {
        let t = s.trim();
        t.eq_ignore_ascii_case(code) || t.eq_ignore_ascii_case(rule) ||
        t.to_ascii_uppercase() == code_upper || t.to_ascii_lowercase() == rule_lower
    })
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
